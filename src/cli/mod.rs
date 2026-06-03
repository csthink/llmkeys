//! CLI 子命令接线(spec S4):把 T1–T5 串成 design D3 的数据流。
//!
//! 本模块持共享助手;各子命令实现在 `list` / `show` / `env` / `code` / `key` / `refresh`。
//!
//! 安全:`show` / `key check` 路径**不取明文**(show 只打印 key_ref 引用;check 走 `exists` 只回 bool)。
//! `env` / `code` 输出含明文 key,但那是其用途(S5),经 stdout 或剪贴板,非红线禁止的落盘/日志/argv。

pub mod code;
pub mod env;
pub mod key;
pub mod list;
pub mod refresh;
pub mod show;

use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Result};

use crate::catalog;
use crate::cred_ref::CredRef;
use crate::model::{Provider, ProvidersFile};
use crate::secret::Secret;

/// 读取并合并三层目录。
pub(crate) fn load_catalog() -> Result<ProvidersFile> {
    catalog::load_merged()
}

/// 按 id 找 provider;找不到给可操作错误。
pub(crate) fn find_provider<'a>(cat: &'a ProvidersFile, id: &str) -> Result<&'a Provider> {
    cat.providers
        .get(id)
        .ok_or_else(|| anyhow!("Provider not found: `{id}`; run `llmkeys list` to see available ones"))
}

/// 由 provider 的 key_ref 解析 CredRef;`--profile` 若给定则覆盖其 profile。
pub(crate) fn resolve_cred(p: &Provider, id: &str, profile: Option<&str>) -> Result<CredRef> {
    let key_ref = p
        .key_ref
        .as_deref()
        .ok_or_else(|| anyhow!("provider `{id}` has no key_ref configured, cannot fetch the key"))?;
    let mut cred =
        CredRef::from_str(key_ref).with_context(|| format!("invalid key_ref for provider `{id}`"))?;
    if let Some(prof) = profile {
        cred.profile = Some(prof.to_string());
    }
    Ok(cred)
}

/// 解析 `key set/check` 的目标 `<id[#profile]>`。
pub(crate) fn parse_key_target(target: &str) -> Result<(String, Option<String>)> {
    match target.split_once('#') {
        Some((id, profile)) => {
            if id.is_empty() {
                bail!("target is missing the provider id (e.g. `openrouter` or `openrouter#work`)");
            }
            if profile.is_empty() {
                bail!("the profile after `#` must not be empty");
            }
            Ok((id.to_string(), Some(profile.to_string())))
        }
        None => {
            if target.is_empty() {
                bail!("target must not be empty (e.g. `openrouter` or `openrouter#work`)");
            }
            Ok((target.to_string(), None))
        }
    }
}

/// 交付渲染结果:片段始终写 **stdout**(数据);`--copy` 时**额外**送剪贴板,
/// 状态提示走 **stderr**(spec S4 的 SHOULD「已复制」)。剪贴板不可用时优雅降级为提示,
/// 不让 `--copy` 失败(片段已在 stdout,用户仍可用)。
pub(crate) fn deliver(snippet: &Secret, copy: bool) -> Result<()> {
    print!("{}", snippet.as_str());
    if copy {
        match arboard::Clipboard::new().and_then(|mut c| c.set_text(snippet.as_str())) {
            Ok(()) => eprintln!("Copied to clipboard."),
            Err(e) => eprintln!("Note: failed to copy to clipboard ({e}); the snippet was printed to stdout above."),
        }
    }
    Ok(())
}

/// 把秒数转成粗略人类可读时长。
pub(crate) fn humanize(secs: u64) -> String {
    if secs < 60 {
        format!("{secs} seconds")
    } else if secs < 3600 {
        format!("{} minutes", secs / 60)
    } else if secs < 86_400 {
        format!("{} hours", secs / 3600)
    } else {
        format!("{} days", secs / 86_400)
    }
}

/// 当前数据来源提示(spec S6:明确提示来源)。
pub(crate) fn data_source_line() -> String {
    use crate::catalog::modelsdev::{status, CacheStatus};
    match status() {
        CacheStatus::Missing => {
            "Source: built-in snapshot + user overrides (no models.dev cache; run `llmkeys refresh` to fetch)".to_string()
        }
        CacheStatus::Fresh { age_secs } => format!(
            "Source: built-in snapshot + models.dev cache ({} ago) + user overrides",
            humanize(age_secs)
        ),
        CacheStatus::Stale { age_secs } => format!(
            "Source: built-in snapshot + models.dev cache ({} ago, stale; consider `llmkeys refresh`) + user overrides",
            humanize(age_secs)
        ),
    }
}
