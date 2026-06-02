//! 配置与缓存路径解析(design D4)。
//!
//! - overrides(用户覆盖层):`~/.config/qiao/providers.toml`
//! - models.dev 缓存:`~/.cache/qiao/modelsdev.json`(拉取时间戳 + TTL 由 T3 处理)
//!
//! D4 同时写了字面量 `~/.config` / `~/.cache` **和**"用 directories crate"。但 `directories`
//! 的 `ProjectDirs` 在 macOS 返回 `~/Library/Application Support` / `~/Library/Caches`,与字面量
//! 冲突。**设计文档(字面量路径)是事实来源**,故这里按 XDG 风格解析 `~/.config` / `~/.cache`,
//! 并尊重 `$XDG_CONFIG_HOME` / `$XDG_CACHE_HOME`(便于测试与 CI 注入);`directories::BaseDirs`
//! 仅用于取 home 目录。v1 只验 macOS,但解析逻辑天然跨平台(为跨平台预留,不实现 Linux 专属分支)。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 应用目录名(config / cache 下的子目录)。
const APP_DIR: &str = "qiao";

/// 用户主目录,经 `directories::BaseDirs` 解析。
fn home_dir() -> Result<PathBuf> {
    directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .context("无法定位用户主目录(HOME 未设置?)")
}

/// 纯函数:按 "XDG 值优先,否则 home/<fallback_sub>" 解析出应用目录。
///
/// 抽出来是为了可在不触碰真实环境变量的前提下单测路径拼接(DoD:路径解析正确)。
fn resolve_app_dir(xdg: Option<&OsStr>, home: &Path, fallback_sub: &str) -> PathBuf {
    match xdg {
        Some(x) if !x.is_empty() => PathBuf::from(x).join(APP_DIR),
        _ => home.join(fallback_sub).join(APP_DIR),
    }
}

/// 配置目录:`$XDG_CONFIG_HOME/qiao` 或 `~/.config/qiao`。
pub fn config_dir() -> Result<PathBuf> {
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    Ok(resolve_app_dir(xdg.as_deref(), &home_dir()?, ".config"))
}

/// 缓存目录:`$XDG_CACHE_HOME/qiao` 或 `~/.cache/qiao`。
pub fn cache_dir() -> Result<PathBuf> {
    let xdg = std::env::var_os("XDG_CACHE_HOME");
    Ok(resolve_app_dir(xdg.as_deref(), &home_dir()?, ".cache"))
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

    #[test]
    fn xdg_value_wins_when_set() {
        let home = Path::new("/Users/alice");
        let dir = resolve_app_dir(Some(OsStr::new("/custom/cfg")), home, ".config");
        assert_eq!(dir, PathBuf::from("/custom/cfg/qiao"));
    }

    #[test]
    fn falls_back_to_home_subdir_when_xdg_absent() {
        let home = Path::new("/Users/alice");
        assert_eq!(
            resolve_app_dir(None, home, ".config"),
            PathBuf::from("/Users/alice/.config/qiao")
        );
        assert_eq!(
            resolve_app_dir(None, home, ".cache"),
            PathBuf::from("/Users/alice/.cache/qiao")
        );
    }

    #[test]
    fn empty_xdg_value_falls_back() {
        // 空字符串视为未设置,回落到 ~/.<sub>。
        let home = Path::new("/Users/alice");
        assert_eq!(
            resolve_app_dir(Some(OsStr::new("")), home, ".config"),
            PathBuf::from("/Users/alice/.config/qiao")
        );
    }

    #[test]
    fn file_names_are_appended() {
        // 验证最终文件名拼接(不依赖真实 home)。
        let home = Path::new("/Users/alice");
        let cfg = resolve_app_dir(None, home, ".config").join("providers.toml");
        let cache = resolve_app_dir(None, home, ".cache").join("modelsdev.json");
        assert!(cfg.ends_with("qiao/providers.toml"));
        assert!(cache.ends_with("qiao/modelsdev.json"));
    }
}
