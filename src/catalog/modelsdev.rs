//! 三层合并的中间层:models.dev 拉取 + 缓存 + TTL + 失败降级(spec S6, design D4)。
//!
//! 缓存文件:`~/.cache/qiao/modelsdev.json`,内容 `{ fetched_at, providers }`。
//!
//! **降级(S6)**:拉取失败 → 保留旧缓存(`refresh` 在成功拉取前绝不动缓存);缓存缺失/损坏 →
//! 空层(合并时退回快照,`list` 仍可用)。任何路径都**不**把 key 写进日志(本层不接触 key)。
//!
//! **v1 贡献范围(PROPOSAL-001,已采纳 A)**:models.dev 的 `api` 字段对国内 provider 是国际端点
//! (siliconflow `.com`、deepseek 缺 `/v1`),若覆盖快照会破坏国内配置,违反 CLAUDE.md「国内
//! provider 以快照为准」。裁断:CLAUDE.md(项目宪法)优先于 spec S3 的通用合并序——故本层 v1
//! **只规范化 `display_name`,不贡献 base_url / 模型选择**,在 v1 **不发挥目录作用**;且合并入口
//! 只用它 enrich 已知 provider(见 `super::load_merged`)。base_url / 模型目录的字段贡献待 schema
//! 成长后再放开(见 `docs/proposals/PROPOSAL-001-modelsdev-scope.md`、spec S3 脚注、design D4)。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config;
use crate::model::{Provider, ProvidersFile};

const MODELS_DEV_URL: &str = "https://models.dev/api.json";
/// 缓存有效期默认 24h(design D4)。
const TTL_SECS: u64 = 24 * 60 * 60;
const FETCH_TIMEOUT: Duration = Duration::from_secs(20);

/// 缓存文件结构:拉取时间戳 + 规范化后的 provider 层。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cache {
    /// 拉取时刻(unix 秒),用于 TTL 判定。
    fetched_at: u64,
    providers: ProvidersFile,
}

/// models.dev 上游 provider:只取 v1 用得到的字段,其余忽略(向前兼容)。
#[derive(Debug, Deserialize)]
struct Upstream {
    #[serde(default)]
    name: Option<String>,
}

/// 缓存新鲜度,供 `refresh` / `list` 明示数据来源(S6)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheStatus {
    /// 无缓存(将退回快照)。
    Missing,
    /// 有缓存且在 TTL 内。
    Fresh { age_secs: u64 },
    /// 有缓存但已过期(仍可用,建议 `qiao refresh`)。
    Stale { age_secs: u64 },
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_cache(path: &Path) -> Result<Option<Cache>> {
    match fs::read_to_string(path) {
        Ok(s) => {
            let c: Cache = serde_json::from_str(&s)
                .with_context(|| format!("缓存解析失败: {}", path.display()))?;
            Ok(Some(c))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("读取缓存失败: {}", path.display())),
    }
}

fn write_cache(path: &Path, cache: &Cache) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("创建缓存目录失败: {}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(cache).context("缓存序列化失败")?;
    // 原子写:先写临时文件再 rename,避免拉取/写入中断留下半截缓存。
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).with_context(|| format!("写临时缓存失败: {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("替换缓存失败: {}", path.display()))?;
    Ok(())
}

/// 把上游 JSON 规范化为我们的 provider 层(v1 只取 `display_name`,见模块说明)。
fn normalize(upstream: BTreeMap<String, Upstream>) -> ProvidersFile {
    let providers = upstream
        .into_iter()
        .map(|(id, up)| {
            let p = Provider {
                display_name: up.name,
                ..Default::default()
            };
            (id, p)
        })
        .collect();
    ProvidersFile { providers }
}

/// 读缓存层供合并使用。缺失或损坏 → 空层(让 `list` 退回快照,S6)。
pub fn load_layer() -> ProvidersFile {
    let Ok(path) = config::modelsdev_cache_path() else {
        return ProvidersFile::default();
    };
    match read_cache(&path) {
        Ok(Some(c)) => c.providers,
        _ => ProvidersFile::default(),
    }
}

/// 当前缓存新鲜度(供 T6 的 `list` / `refresh` 提示数据来源)。
pub fn status() -> CacheStatus {
    let Ok(path) = config::modelsdev_cache_path() else {
        return CacheStatus::Missing;
    };
    match read_cache(&path) {
        Ok(Some(c)) => {
            let age = now_unix().saturating_sub(c.fetched_at);
            if age <= TTL_SECS {
                CacheStatus::Fresh { age_secs: age }
            } else {
                CacheStatus::Stale { age_secs: age }
            }
        }
        _ => CacheStatus::Missing,
    }
}

/// 拉取 models.dev 并更新缓存。失败时返回 `Err` 且**不触碰旧缓存**(S6:保留旧缓存)。
pub fn refresh() -> Result<ProvidersFile> {
    let path = config::modelsdev_cache_path()?;
    persist_fetched(&path, fetch, now_unix())
}

/// 实际 HTTP 拉取 + 规范化。网络/状态/解析任一失败即 `Err`。
fn fetch() -> Result<ProvidersFile> {
    let client = reqwest::blocking::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .context("构建 HTTP 客户端失败")?;
    let upstream: BTreeMap<String, Upstream> = client
        .get(MODELS_DEV_URL)
        .send()
        .context("拉取 models.dev 失败(网络不可达?)")?
        .error_for_status()
        .context("models.dev 返回错误状态")?
        .json()
        .context("解析 models.dev 响应失败")?;
    Ok(normalize(upstream))
}

/// 把"拉取结果"持久化:**只有成功拉取**才写缓存。拉取 `Err` 时直接返回,旧缓存原封不动。
/// 抽出 `fetch_fn` 参数以便不依赖网络地单测降级行为。
fn persist_fetched(
    path: &Path,
    fetch_fn: impl FnOnce() -> Result<ProvidersFile>,
    now: u64,
) -> Result<ProvidersFile> {
    let providers = fetch_fn()?;
    let cache = Cache {
        fetched_at: now,
        providers: providers.clone(),
    };
    write_cache(path, &cache)?;
    Ok(providers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("qiao-test-modelsdev-{name}.json"))
    }

    #[test]
    fn normalize_takes_display_name_only() {
        let json = r#"{
            "openrouter": { "name": "OpenRouter", "api": "https://openrouter.ai/api/v1" },
            "deepseek":   { "name": "DeepSeek",   "api": "https://api.deepseek.com" }
        }"#;
        let upstream: BTreeMap<String, Upstream> = serde_json::from_str(json).unwrap();
        let layer = normalize(upstream);
        assert_eq!(
            layer.providers["openrouter"].display_name.as_deref(),
            Some("OpenRouter")
        );
        // base_url 不从 models.dev 取(避免国内端点污染)。
        assert_eq!(layer.providers["deepseek"].base_url, None);
    }

    #[test]
    fn cache_round_trips() {
        let path = tmp("roundtrip");
        let _ = fs::remove_file(&path);
        let mut providers = ProvidersFile::default();
        providers.providers.insert(
            "openrouter".into(),
            Provider {
                display_name: Some("OpenRouter".into()),
                ..Default::default()
            },
        );
        persist_fetched(&path, || Ok(providers.clone()), 123).unwrap();

        let c = read_cache(&path).unwrap().unwrap();
        assert_eq!(c.fetched_at, 123);
        assert_eq!(
            c.providers.providers["openrouter"].display_name.as_deref(),
            Some("OpenRouter")
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn failed_refresh_keeps_old_cache() {
        let path = tmp("keep-old");
        // 先放一份"旧缓存"。
        let mut old = ProvidersFile::default();
        old.providers.insert(
            "marker".into(),
            Provider {
                display_name: Some("OLD".into()),
                ..Default::default()
            },
        );
        persist_fetched(&path, || Ok(old.clone()), 1).unwrap();

        // 模拟拉取失败:persist 必须不写,旧缓存原封不动。
        let res = persist_fetched(&path, || Err(anyhow!("network down")), 999);
        assert!(res.is_err());

        let c = read_cache(&path).unwrap().unwrap();
        assert_eq!(c.fetched_at, 1, "旧缓存的时间戳应保持");
        assert!(c.providers.providers.contains_key("marker"));
        assert_eq!(
            c.providers.providers["marker"].display_name.as_deref(),
            Some("OLD")
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_cache_loads_empty() {
        let path = tmp("none");
        let _ = fs::remove_file(&path);
        assert!(read_cache(&path).unwrap().is_none());
    }

    #[test]
    fn corrupt_cache_read_errors() {
        let path = tmp("corrupt");
        fs::write(&path, "{ not valid json ").unwrap();
        assert!(read_cache(&path).is_err());
        let _ = fs::remove_file(&path);
    }
}
