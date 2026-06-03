//! 三层合并的最高层:用户 overrides(`~/.config/llmkeys/providers.toml`,spec S3)。
//!
//! **用户写的永远赢**。文件不存在不是错误(返回空层);存在但解析失败才报错。

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::config;
use crate::model::ProvidersFile;

/// 读用户 overrides 层(缺文件 → 空层)。
pub fn load() -> Result<ProvidersFile> {
    load_from(&config::overrides_path()?)
}

/// 从指定路径读取(便于测试与复用)。文件不存在 → 空层。
fn load_from(path: &Path) -> Result<ProvidersFile> {
    match fs::read_to_string(path) {
        Ok(s) => toml::from_str(&s)
            .with_context(|| format!("failed to parse user overrides: {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProvidersFile::default()),
        Err(e) => Err(e).with_context(|| format!("failed to read user overrides: {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("llmkeys-test-overrides-{name}.toml"))
    }

    #[test]
    fn missing_file_is_empty_layer() {
        let path = tmp("missing-xyz");
        let _ = fs::remove_file(&path);
        let f = load_from(&path).unwrap();
        assert!(f.providers.is_empty());
    }

    #[test]
    fn parses_partial_override() {
        let path = tmp("partial");
        fs::write(
            &path,
            "[providers.openrouter]\nbase_url = \"https://my-proxy.local/v1\"\n",
        )
        .unwrap();
        let f = load_from(&path).unwrap();
        assert_eq!(
            f.providers["openrouter"].base_url.as_deref(),
            Some("https://my-proxy.local/v1")
        );
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn malformed_file_errors() {
        let path = tmp("malformed");
        fs::write(&path, "this is = = not toml [[[").unwrap();
        assert!(load_from(&path).is_err());
        let _ = fs::remove_file(&path);
    }
}
