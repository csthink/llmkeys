//! 配置与缓存路径解析(design D4)。
//!
//! - overrides(用户覆盖层):`~/.config/llmkeys/providers.toml`
//! - models.dev 缓存:`~/.cache/llmkeys/modelsdev.json`(拉取时间戳 + TTL 由 T3 处理)
//!
//! D4 同时写了字面量 `~/.config` / `~/.cache` **和**"用 directories crate"。但 `directories`
//! 的 `ProjectDirs` 在 macOS 返回 `~/Library/Application Support` / `~/Library/Caches`,与字面量
//! 冲突。**设计文档(字面量路径)是事实来源**,故这里按 XDG 风格解析 `~/.config` / `~/.cache`,
//! 并尊重 `$XDG_CONFIG_HOME` / `$XDG_CACHE_HOME`(便于测试与 CI 注入);`directories::BaseDirs`
//! 仅用于取 home 目录。v1 只验 macOS,但解析逻辑天然跨平台(为跨平台预留,不实现 Linux 专属分支)。

use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// 应用目录名(config / cache 下的子目录)。
const APP_DIR: &str = "llmkeys";

/// 用户主目录,经 `directories::BaseDirs` 解析。
fn home_dir() -> Result<PathBuf> {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .context("无法定位用户主目录(HOME 未设置?)")
}

/// 按 "XDG 值优先,否则 home/<fallback_sub>" 解析应用目录。
///
/// `home` **惰性求值**:仅在需要回退时才解析,XDG 已设则根本不触碰 home(故 HOME 缺失
/// 但 XDG 已设的边缘环境不会无谓报错)。闭包形式同时让单测可注入合成 home、不碰真实环境。
fn resolve_app_dir(
    xdg: Option<&OsStr>,
    fallback_sub: &str,
    home: impl FnOnce() -> Result<PathBuf>,
) -> Result<PathBuf> {
    match xdg {
        Some(x) if !x.is_empty() => Ok(PathBuf::from(x).join(APP_DIR)),
        _ => Ok(home()?.join(fallback_sub).join(APP_DIR)),
    }
}

/// 配置目录:`$XDG_CONFIG_HOME/llmkeys` 或 `~/.config/llmkeys`。
pub fn config_dir() -> Result<PathBuf> {
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    resolve_app_dir(xdg.as_deref(), ".config", home_dir)
}

/// 缓存目录:`$XDG_CACHE_HOME/llmkeys` 或 `~/.cache/llmkeys`。
pub fn cache_dir() -> Result<PathBuf> {
    let xdg = std::env::var_os("XDG_CACHE_HOME");
    resolve_app_dir(xdg.as_deref(), ".cache", home_dir)
}

/// 用户 overrides 文件:`<config_dir>/providers.toml`。
pub fn overrides_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("providers.toml"))
}

/// models.dev 缓存文件:`<cache_dir>/modelsdev.json`。
pub fn modelsdev_cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("modelsdev.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合成 home(不碰真实环境)。
    fn alice() -> Result<PathBuf> {
        Ok(PathBuf::from("/Users/alice"))
    }

    #[test]
    fn xdg_value_wins_when_set() {
        let dir = resolve_app_dir(Some(OsStr::new("/custom/cfg")), ".config", alice).unwrap();
        assert_eq!(dir, PathBuf::from("/custom/cfg/llmkeys"));
    }

    #[test]
    fn home_not_resolved_when_xdg_set() {
        // 坐实惰性:XDG 已设时绝不调用 home 解析(否则 panic 会让测试失败)。
        let dir = resolve_app_dir(Some(OsStr::new("/custom")), ".config", || {
            panic!("XDG 已设时不应解析 home")
        })
        .unwrap();
        assert_eq!(dir, PathBuf::from("/custom/llmkeys"));
    }

    #[test]
    fn falls_back_to_home_subdir_when_xdg_absent() {
        assert_eq!(
            resolve_app_dir(None, ".config", alice).unwrap(),
            PathBuf::from("/Users/alice/.config/llmkeys")
        );
        assert_eq!(
            resolve_app_dir(None, ".cache", alice).unwrap(),
            PathBuf::from("/Users/alice/.cache/llmkeys")
        );
    }

    #[test]
    fn empty_xdg_value_falls_back() {
        // 空字符串视为未设置,回落到 ~/.<sub>。
        assert_eq!(
            resolve_app_dir(Some(OsStr::new("")), ".config", alice).unwrap(),
            PathBuf::from("/Users/alice/.config/llmkeys")
        );
    }

    #[test]
    fn file_names_are_appended() {
        // 验证最终文件名拼接(不依赖真实 home)。
        let cfg = resolve_app_dir(None, ".config", alice).unwrap().join("providers.toml");
        let cache = resolve_app_dir(None, ".cache", alice).unwrap().join("modelsdev.json");
        assert!(cfg.ends_with("llmkeys/providers.toml"));
        assert!(cache.ends_with("llmkeys/modelsdev.json"));
    }
}
