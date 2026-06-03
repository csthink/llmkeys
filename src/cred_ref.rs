//! 凭证引用 URI 解析(spec S1)。
//!
//! 配置里**只存引用,绝不存 key 本身**。语法:
//!
//! ```text
//! <backend>:<locator>[#<profile>]
//! ```
//!
//! - `<backend>`:`keychain` | `bw` | `env`(三者之一,**绝不**含 `bws`,见 CLAUDE.md 红线 3)。
//! - `<locator>`:含义随 backend 而定,内部允许再出现 `/`(如 `item/带 空格 的名字`)。
//! - `#<profile>`:可选,多账号区分(个人号 / 工作号)。
//!
//! 解析规则(MUST,见 S1):
//! - 以**第一个 `:`** 切分 backend 与剩余部分。
//! - `#profile` 从**末尾**切分;v1 不支持 locator 内含 `#` 的转义(留作 later)。
//! - 非法 backend、空 locator、空 profile 一律报清晰错误,不静默失败、不 panic。
//!
//! 安全:本模块只处理"引用",**不接触明文 key**。错误类型的 `Display` 仅含引用本身的
//! 结构信息(backend 名 / 原始引用串),引用串永不含 key,故不违反安全红线。

use std::fmt;
use std::str::FromStr;

/// 密钥后端。v1 仅三者;**绝不**加入 `bws`(Secrets Manager,见红线 3)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Keychain,
    Bw,
    Env,
}

impl Backend {
    /// 规范名,用于回显与 `Display`(与解析时接受的字面量一致)。
    pub fn as_str(self) -> &'static str {
        match self {
            Backend::Keychain => "keychain",
            Backend::Bw => "bw",
            Backend::Env => "env",
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 一条凭证引用,解析自 `<backend>:<locator>[#profile]`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredRef {
    pub backend: Backend,
    /// backend 特定定位符(keychain: provider 名;bw: `item/名` 或 `id/...`;env: 变量名)。
    pub locator: String,
    pub profile: Option<String>,
}

impl fmt::Display for CredRef {
    /// 回显为规范引用串(`show` 用其展示 key 引用形式,绝不展示明文)。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.backend, self.locator)?;
        if let Some(profile) = &self.profile {
            write!(f, "#{profile}")?;
        }
        Ok(())
    }
}

/// 解析失败原因。所有变体的 `Display` 均为人类可读消息,且**不含明文 key**
/// (输入是配置中的引用串,引用串永不含 key)。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CredRefError {
    #[error("credential reference is missing the ':' separator: expected `<backend>:<locator>[#profile]`, got `{0}`")]
    MissingColon(String),

    #[error("unknown key backend `{0}`: only keychain | bw | env are supported")]
    UnknownBackend(String),

    #[error("the credential reference locator is empty: there must be content after `<backend>:`")]
    EmptyLocator,

    #[error("the credential reference profile is empty: there must be content after `#` (if the locator itself needs '#', v1 does not support that yet)")]
    EmptyProfile,
}

impl FromStr for Backend {
    type Err = CredRefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "keychain" => Ok(Backend::Keychain),
            "bw" => Ok(Backend::Bw),
            "env" => Ok(Backend::Env),
            other => Err(CredRefError::UnknownBackend(other.to_string())),
        }
    }
}

impl FromStr for CredRef {
    type Err = CredRefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 1) 第一个 ':' 切 backend 与剩余;locator 内允许再含 ':'/'/' 故只切首个。
        let (backend_str, rest) = s
            .split_once(':')
            .ok_or_else(|| CredRefError::MissingColon(s.to_string()))?;

        let backend = backend_str.parse::<Backend>()?;

        // 2) profile 从**末尾**切;locator 内的 '#' 因此不会被误当 profile 起点
        //    (v1 不支持转义,locator 含 '#' 的极端情形留 later)。
        let (locator, profile) = match rest.rsplit_once('#') {
            Some((loc, prof)) => {
                if prof.is_empty() {
                    return Err(CredRefError::EmptyProfile);
                }
                (loc, Some(prof.to_string()))
            }
            None => (rest, None),
        };

        // 3) locator 不得为空(`keychain:`、`keychain:#work` 等都在此拦下)。
        if locator.is_empty() {
            return Err(CredRefError::EmptyLocator);
        }

        Ok(CredRef {
            backend,
            locator: locator.to_string(),
            profile,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 解析便捷封装。
    fn parse(s: &str) -> Result<CredRef, CredRefError> {
        s.parse()
    }

    // —— S1 合法示例表:逐行覆盖 ——

    #[test]
    fn keychain_default_profile() {
        let r = parse("keychain:openrouter").unwrap();
        assert_eq!(r.backend, Backend::Keychain);
        assert_eq!(r.locator, "openrouter");
        assert_eq!(r.profile, None);
    }

    #[test]
    fn keychain_with_profile() {
        let r = parse("keychain:openrouter#work").unwrap();
        assert_eq!(r.backend, Backend::Keychain);
        assert_eq!(r.locator, "openrouter");
        assert_eq!(r.profile.as_deref(), Some("work"));
    }

    #[test]
    fn bw_item_by_name_with_slash_and_spaces() {
        // locator 含 '/' 与空格,必须原样保留。
        let r = parse("bw:item/OpenRouter API Key").unwrap();
        assert_eq!(r.backend, Backend::Bw);
        assert_eq!(r.locator, "item/OpenRouter API Key");
        assert_eq!(r.profile, None);
    }

    #[test]
    fn bw_item_by_id() {
        let r = parse("bw:id/2a16-445b-...").unwrap();
        assert_eq!(r.backend, Backend::Bw);
        assert_eq!(r.locator, "id/2a16-445b-...");
        assert_eq!(r.profile, None);
    }

    #[test]
    fn env_variable() {
        let r = parse("env:OPENROUTER_API_KEY").unwrap();
        assert_eq!(r.backend, Backend::Env);
        assert_eq!(r.locator, "OPENROUTER_API_KEY");
        assert_eq!(r.profile, None);
    }

    // —— 非法输入(≥3 个):每个都须报清晰错误,不 panic ——

    #[test]
    fn err_missing_colon() {
        assert_eq!(
            parse("openrouter"),
            Err(CredRefError::MissingColon("openrouter".to_string()))
        );
    }

    #[test]
    fn err_unknown_backend() {
        assert_eq!(
            parse("vault:something"),
            Err(CredRefError::UnknownBackend("vault".to_string()))
        );
    }

    #[test]
    fn err_empty_locator() {
        assert_eq!(parse("keychain:"), Err(CredRefError::EmptyLocator));
    }

    #[test]
    fn err_empty_locator_with_profile() {
        // `#work` 是 profile,切掉后 locator 为空。
        assert_eq!(parse("keychain:#work"), Err(CredRefError::EmptyLocator));
    }

    #[test]
    fn err_empty_profile() {
        assert_eq!(parse("keychain:openrouter#"), Err(CredRefError::EmptyProfile));
    }

    // —— 红线 3:`bws` 必须被当作未知后端拒绝,绝不放行 ——

    #[test]
    fn rejects_bws_backend() {
        assert_eq!(
            parse("bws:secret/id"),
            Err(CredRefError::UnknownBackend("bws".to_string()))
        );
    }

    // —— 边界:profile 从末尾切,locator 内的 ':' 不被二次切分 ——

    #[test]
    fn first_colon_only_split() {
        // locator 含 ':'(如某些 id 形态),只切首个 ':'。
        let r = parse("bw:id/abc:def").unwrap();
        assert_eq!(r.backend, Backend::Bw);
        assert_eq!(r.locator, "id/abc:def");
    }

    #[test]
    fn profile_split_from_end() {
        // 仅最后一个 '#' 作为 profile 分隔(此例 locator 不含 '#',验证多段时取末尾)。
        let r = parse("bw:item/My Key#work").unwrap();
        assert_eq!(r.locator, "item/My Key");
        assert_eq!(r.profile.as_deref(), Some("work"));
    }

    // —— Display 往返:解析后回显应得规范引用串 ——

    #[test]
    fn display_round_trip() {
        for s in [
            "keychain:openrouter",
            "keychain:openrouter#work",
            "bw:item/OpenRouter API Key",
            "env:OPENROUTER_API_KEY",
        ] {
            assert_eq!(parse(s).unwrap().to_string(), s);
        }
    }
}
