//! Provider / Models 数据结构(spec S3)。
//!
//! 这是配置的 **serde 表示**,用于读取三层中的任意一层(内置快照 / models.dev 缓存 /
//! 用户 overrides),供 catalog 模块(T3)做 field-level 合并。
//!
//! 两条契约(DoD / spec S3):
//! - **未知字段不报错**:不使用 `#[serde(deny_unknown_fields)]`,serde 默认忽略多余字段,
//!   保证向前兼容(上游新增字段不致解析失败)。
//! - **未知模型角色必须保留**:`models` 表用 `#[serde(flatten)]` 收进 map,
//!   v1 识别 `chat`/`embedding`,任何未知角色原样留存,不丢弃、不报错。
//!
//! 字段为何是 `Option`:S3 规定 `display_name`/`base_url`/`key_ref`/`env_prefix` 为 MUST,
//! 但那是对**合并后最终 provider** 的要求。单独某一层(尤其 overrides)允许只覆盖一个字段,
//! 故每层的字段须可缺省。MUST-存在性校验在合并点(T3/T6)做,缺失时给出
//! "指明 provider + 字段名" 的错误(spec S3),而非在反序列化阶段。
//!
//! 安全:本结构只承载**非机密配置**;`key_ref` 仅是"去哪取 key"的引用串,绝不含 key 明文。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 一个 providers 配置文件(或缓存层)的顶层表示:`[providers.<id>] ...`。
///
/// 对应 TOML:
/// ```toml
/// [providers.openrouter]
/// base_url = "..."
///   [providers.openrouter.models]
///   chat = "..."
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProvidersFile {
    /// provider id → Provider。用 `BTreeMap` 保证遍历/输出顺序确定(便于 `list` 与测试)。
    #[serde(default)]
    pub providers: BTreeMap<String, Provider>,
}

/// 单个 provider 的配置(某一层的视图;字段可缺省,见模块文档)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Provider {
    /// 人类可读名,如 "OpenRouter"(S3 MUST,合并后校验)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// OpenAI 兼容 API 基址(S3 MUST)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// 凭证引用 `<backend>:<locator>[#profile]`(S3 MUST);**只存引用,绝不存 key**。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_ref: Option<String>,
    /// `.env` 输出变量前缀(S3 MUST),如 "OPENROUTER"。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_prefix: Option<String>,
    /// 模型命名提示(S3 SHOULD),人类可读。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub naming_note: Option<String>,
    /// 模型角色表(chat / embedding / 未知角色全部保留)。
    #[serde(default, skip_serializing_if = "Models::is_empty")]
    pub models: Models,
}

/// 模型角色 → 模型 ID 的映射。
///
/// v1 显式识别 `chat`、`embedding`(见 [`Models::chat`] / [`Models::embedding`]),
/// 但底层用单一 map 承载**所有**角色,未知角色原样保留(spec S3 向前兼容)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Models {
    /// 角色名 → 模型 ID。`BTreeMap` 保证顺序确定。
    #[serde(flatten)]
    pub roles: BTreeMap<String, String>,
}

impl Models {
    /// `chat` 角色对应的模型 ID(若有)。
    pub fn chat(&self) -> Option<&str> {
        self.roles.get("chat").map(String::as_str)
    }

    /// `embedding` 角色对应的模型 ID(若有);DeepSeek 等无 embedding 的 provider 返回 `None`。
    pub fn embedding(&self) -> Option<&str> {
        self.roles.get("embedding").map(String::as_str)
    }

    /// 是否未配置任何角色。
    pub fn is_empty(&self) -> bool {
        self.roles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 内置快照在编译期嵌入,确保 D5 样例始终可反序列化(DoD)。
    const SNAPSHOT: &str = include_str!("../snapshot/providers.snapshot.toml");

    #[test]
    fn deserializes_d5_snapshot() {
        let file: ProvidersFile = toml::from_str(SNAPSHOT).expect("快照应能反序列化");
        // D5 预置 4 家。
        assert_eq!(file.providers.len(), 4);
        for id in ["openrouter", "siliconflow", "aliyun_bailian", "deepseek"] {
            assert!(file.providers.contains_key(id), "缺 provider: {id}");
        }
    }

    #[test]
    fn openrouter_fields_and_models() {
        let file: ProvidersFile = toml::from_str(SNAPSHOT).unwrap();
        let p = &file.providers["openrouter"];
        assert_eq!(p.base_url.as_deref(), Some("https://openrouter.ai/api/v1"));
        assert_eq!(p.key_ref.as_deref(), Some("keychain:openrouter"));
        assert_eq!(p.env_prefix.as_deref(), Some("OPENROUTER"));
        assert_eq!(p.models.chat(), Some("openai/gpt-5.5"));
        assert_eq!(p.models.embedding(), Some("baai/bge-m3"));
    }

    #[test]
    fn deepseek_has_no_embedding() {
        let file: ProvidersFile = toml::from_str(SNAPSHOT).unwrap();
        let p = &file.providers["deepseek"];
        assert_eq!(p.models.chat(), Some("deepseek-v4-pro"));
        assert_eq!(p.models.embedding(), None);
    }

    #[test]
    fn unknown_model_role_is_preserved() {
        // 未知角色 `rerank` 必须保留不报错(向前兼容)。
        let toml = r#"
            [providers.x]
            base_url = "https://example.com/v1"
            [providers.x.models]
            chat = "m-chat"
            rerank = "m-rerank"
        "#;
        let file: ProvidersFile = toml::from_str(toml).unwrap();
        let m = &file.providers["x"].models;
        assert_eq!(m.chat(), Some("m-chat"));
        assert_eq!(m.roles.get("rerank").map(String::as_str), Some("m-rerank"));
    }

    #[test]
    fn unknown_top_level_field_does_not_error() {
        // provider 上多出未知字段不应导致解析失败。
        let toml = r#"
            [providers.x]
            base_url = "https://example.com/v1"
            future_field = "ignored"
        "#;
        let file: ProvidersFile = toml::from_str(toml).unwrap();
        assert_eq!(
            file.providers["x"].base_url.as_deref(),
            Some("https://example.com/v1")
        );
    }

    #[test]
    fn partial_layer_omits_most_fields() {
        // overrides 层只覆盖一个字段:其余字段为 None,不报错(支撑 T3 field-level 合并)。
        let toml = r#"
            [providers.openrouter]
            base_url = "https://my-proxy.local/v1"
        "#;
        let file: ProvidersFile = toml::from_str(toml).unwrap();
        let p = &file.providers["openrouter"];
        assert_eq!(p.base_url.as_deref(), Some("https://my-proxy.local/v1"));
        assert_eq!(p.key_ref, None);
        assert!(p.models.is_empty());
    }

    #[test]
    fn empty_file_yields_no_providers() {
        let file: ProvidersFile = toml::from_str("").unwrap();
        assert!(file.providers.is_empty());
    }
}
