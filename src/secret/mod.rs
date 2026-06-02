//! 密钥后端:`SecretStore` trait + keychain / bw / env 三实现(design D3,安全 D6/spec S2)。
//!
//! 安全红线(全模块遵守):
//! - 取出的 key 全程用 [`Secret`](`Zeroizing<String>`) 包裹,离开作用域即擦除。
//! - key **绝不**进 argv / 日志 / 持久化 env / 文件。`bw` 的 locator(条目名/ id)进 argv,
//!   但那不是 key;key 只经 `bw` 的 stdout 返回。
//! - 任何 `Err` 的 Display **不含** key(失败信息只取 stderr / 错误类型,二者不含明文)。
//! - `exists`(供 `key check`)只回 `bool`;即便后端 API 必须读出明文判存在,也立即用
//!   `Zeroizing` 接管并丢弃,绝不返回 / 打印。

pub mod bw;
pub mod env;
pub mod keychain;

use anyhow::Result;
use zeroize::Zeroizing;

use crate::cred_ref::{Backend, CredRef};

/// 取出的明文 key:离开作用域自动清零。
pub type Secret = Zeroizing<String>;

/// 可插拔密钥后端。
pub trait SecretStore {
    /// 取明文 key(包在 [`Secret`] 里)。取不到时返回**可操作**的人类可读错误。
    fn get(&self, r: &CredRef) -> Result<Secret>;
    /// 写入 key。某些后端(bw / env)v1 不支持写入,返回带指引的错误。
    fn set(&self, r: &CredRef, value: Secret) -> Result<()>;
    /// 判断 key 是否存在,**只回 bool**,不返回 / 打印明文。
    fn exists(&self, r: &CredRef) -> Result<bool>;
}

/// 按 `r.backend` 选择后端实现。
pub fn store_for(r: &CredRef) -> Box<dyn SecretStore> {
    match r.backend {
        Backend::Keychain => Box::new(keychain::KeychainStore),
        Backend::Bw => Box::new(bw::BwStore),
        Backend::Env => Box::new(env::EnvStore),
    }
}
