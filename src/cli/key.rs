//! `llmkeys key set/check <id[#profile]>`:管理 **keychain** 中的密钥(spec S4)。
//!
//! 二者都作用于 keychain(与子命令语义一致)。`set` 经隐藏输入读 key(不进 argv/history);
//! `check` 只回 yes/no,不打印明文。

use anyhow::{bail, Result};
use zeroize::Zeroizing;

use crate::cred_ref::{Backend, CredRef};
use crate::secret;

/// 由 `<id[#profile]>` 构造 keychain CredRef。
fn keychain_cred(target: &str) -> Result<CredRef> {
    let (id, profile) = super::parse_key_target(target)?;
    Ok(CredRef {
        backend: Backend::Keychain,
        locator: id,
        profile,
    })
}

pub fn set(target: String) -> Result<()> {
    let cred = keychain_cred(&target)?;

    // 隐藏输入:key 不经 argv / shell history。读入即 Zeroizing 托管。
    let prompt = format!("粘贴 {cred} 的 API key(输入不回显,回车确认): ");
    let key = Zeroizing::new(rpassword::prompt_password(prompt)?);
    if key.trim().is_empty() {
        bail!("未输入 key,已取消。");
    }

    secret::store_for(&cred).set(&cred, key)?;
    println!("已写入:{cred}");
    Ok(())
}

pub fn check(target: String) -> Result<()> {
    let cred = keychain_cred(&target)?;
    // exists 只回 bool,不取出 / 打印明文。
    let exists = secret::store_for(&cred).exists(&cred)?;
    println!("{}", if exists { "yes" } else { "no" });
    Ok(())
}
