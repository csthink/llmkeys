//! env 后端:从环境变量取 key(CI / 测试兜底,design D3 / spec S6)。
//!
//! locator 即变量名。**只读兜底**:不支持写入 —— 把 key 写进持久化环境变量违反安全红线
//! (CLAUDE.md:key 绝不进任何持久化的环境变量)。

use anyhow::{anyhow, Result};
use zeroize::Zeroizing;

use super::{Secret, SecretStore};
use crate::cred_ref::CredRef;

pub struct EnvStore;

impl SecretStore for EnvStore {
    fn get(&self, r: &CredRef) -> Result<Secret> {
        let name = &r.locator;
        match std::env::var(name) {
            Ok(v) => Ok(Zeroizing::new(v)),
            // 只提变量名(非 key),不泄露明文。
            Err(_) => Err(anyhow!("环境变量 {name} 未设置;请先 `export {name}=<key>`")),
        }
    }

    fn set(&self, _r: &CredRef, _value: Secret) -> Result<()> {
        // 安全红线:不把 key 写进持久化 env。进程内 set_var 也无意义(子进程才可见且易泄露)。
        Err(anyhow!(
            "env 后端是只读兜底,不支持写入;请改用 keychain(`qiao key set <id>`)或自行 `export`"
        ))
    }

    fn exists(&self, r: &CredRef) -> Result<bool> {
        // var_os 只判存在即丢弃,不读出 / 打印明文。
        Ok(std::env::var_os(&r.locator).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cred_ref::Backend;

    fn cred(name: &str) -> CredRef {
        CredRef {
            backend: Backend::Env,
            locator: name.to_string(),
            profile: None,
        }
    }

    #[test]
    fn get_and_exists_read_env_var() {
        // 用一个测试专属变量名,避免与真实 key 变量冲突。
        let name = "QIAO_TEST_ENV_BACKEND_VAR";
        // SAFETY: 测试内单线程设置/清理自有的测试变量。
        unsafe { std::env::set_var(name, "dummy-not-a-real-key") };

        let store = EnvStore;
        let r = cred(name);
        assert!(store.exists(&r).unwrap());
        assert_eq!(&*store.get(&r).unwrap(), "dummy-not-a-real-key");

        unsafe { std::env::remove_var(name) };
        assert!(!store.exists(&r).unwrap());
        assert!(store.get(&r).is_err());
    }

    #[test]
    fn set_is_unsupported() {
        let err = EnvStore
            .set(&cred("WHATEVER"), Zeroizing::new("k".into()))
            .unwrap_err();
        assert!(err.to_string().contains("只读"));
    }
}
