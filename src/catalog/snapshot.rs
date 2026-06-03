//! 三层合并的最低层:内置 provider 快照(spec S3 / design D5)。
//!
//! 快照在**编译期**嵌入二进制,保证首次运行 / 离线时永远有兜底数据(DoD:无网络 `list` 仍可用)。

use anyhow::{Context, Result};

use crate::model::ProvidersFile;

/// 编译期嵌入的快照 TOML(运行时资源,见 `snapshot/providers.snapshot.toml`)。
const SNAPSHOT_TOML: &str = include_str!("../../snapshot/providers.snapshot.toml");

/// 解析内置快照层。理论上不会运行时失败(资源随二进制编译进来),
/// 仍返回 `Result` 以统一接口;解析失败意味着构建期资源损坏。
pub fn load() -> Result<ProvidersFile> {
    toml::from_str(SNAPSHOT_TOML).context("failed to parse the built-in snapshot (corrupt build resource?)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_loads_four_providers() {
        let f = load().unwrap();
        assert_eq!(f.providers.len(), 4);
        for id in ["openrouter", "siliconflow", "aliyun_bailian", "deepseek"] {
            assert!(f.providers.contains_key(id));
        }
    }
}
