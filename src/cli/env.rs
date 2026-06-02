//! `llmkeys env <id> [--profile p] [--copy]`:输出 `.env` 片段(spec S4/S5)。

use anyhow::Result;

use crate::{render, secret};

pub fn run(id: String, profile: Option<String>, copy: bool) -> Result<()> {
    let catalog = super::load_catalog()?;
    let p = super::find_provider(&catalog, &id)?;
    let cred = super::resolve_cred(p, &id, profile.as_deref())?;

    // 取明文 key(Zeroizing);后端在取不到时给出可操作错误(keychain 无条目 / bw 未登录)。
    let key = secret::store_for(&cred).get(&cred)?;
    let snippet = render::dotenv(&id, p, &key)?;
    super::deliver(&snippet, copy)
}
