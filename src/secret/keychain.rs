//! keychain 后端:macOS 登录钥匙串(spec S2,design D3)。
//!
//! 条目 = `(service = "dev.mars.qiao", account = "<provider>[#profile]")`,一条目一 key。

use anyhow::{Context, Result};
use zeroize::Zeroizing;

use super::{Secret, SecretStore};
use crate::cred_ref::CredRef;

/// keychain service 标识(spec S2,固定)。
const SERVICE: &str = "dev.mars.qiao";

pub struct KeychainStore;

/// 由 CredRef 拼 keychain account:`<locator>[#profile]`(spec S2)。
fn account(r: &CredRef) -> String {
    match &r.profile {
        Some(profile) => format!("{}#{}", r.locator, profile),
        None => r.locator.clone(),
    }
}

fn entry(r: &CredRef) -> Result<keyring::Entry> {
    let account = account(r);
    keyring::Entry::new(SERVICE, &account)
        .with_context(|| format!("无法打开 keychain 条目(account={account})"))
}

impl SecretStore for KeychainStore {
    fn get(&self, r: &CredRef) -> Result<Secret> {
        let account = account(r);
        match entry(r)?.get_password() {
            Ok(pw) => Ok(Zeroizing::new(pw)),
            Err(keyring::Error::NoEntry) => Err(anyhow::anyhow!(
                "keychain 中没有 {account} 的 key;请先运行 `qiao key set {account}`"
            )),
            // 注意:keyring 错误不含明文 key,可安全透出。
            Err(e) => Err(e).with_context(|| format!("读取 keychain 失败(account={account})")),
        }
    }

    fn set(&self, r: &CredRef, value: Secret) -> Result<()> {
        let account = account(r);
        entry(r)?
            .set_password(&value)
            .with_context(|| format!("写入 keychain 失败(account={account})"))
    }

    fn exists(&self, r: &CredRef) -> Result<bool> {
        let account = account(r);
        match entry(r)?.get_password() {
            // 立即用 Zeroizing 接管明文并丢弃:只判存在,不返回 / 打印。
            Ok(pw) => {
                drop(Zeroizing::new(pw));
                Ok(true)
            }
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(e).with_context(|| format!("查询 keychain 失败(account={account})")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cred_ref::Backend;

    fn cred(locator: &str, profile: Option<&str>) -> CredRef {
        CredRef {
            backend: Backend::Keychain,
            locator: locator.to_string(),
            profile: profile.map(str::to_string),
        }
    }

    #[test]
    fn account_layout_matches_s2() {
        assert_eq!(account(&cred("openrouter", None)), "openrouter");
        assert_eq!(account(&cred("openrouter", Some("work"))), "openrouter#work");
    }

    /// 本机往返手测(DoD):默认 `#[ignore]`,以免普通 `cargo test` 触发钥匙串授权弹窗。
    /// 运行:`cargo test --lib -- --ignored keychain_roundtrip`(会在真实登录钥匙串
    /// 写入/读取/删除一个 `dev.mars.qiao` / `qiao-selftest` 测试条目,用后即删)。
    #[test]
    #[ignore]
    fn keychain_roundtrip() {
        let r = cred("qiao-selftest", None);
        let store = KeychainStore;

        store.set(&r, Zeroizing::new("test-secret-value".to_string())).unwrap();
        assert!(store.exists(&r).unwrap());
        assert_eq!(&*store.get(&r).unwrap(), "test-secret-value");

        // 清理:删除测试条目(直接走 keyring,trait 无 delete)。
        entry(&r).unwrap().delete_credential().unwrap();
        assert!(!store.exists(&r).unwrap());
    }
}
