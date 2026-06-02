//! `llmkeys code <id> [--profile p] [--copy]`:输出 LangChain 代码片段(spec S4/S5)。

use anyhow::Result;

use crate::{render, secret};

pub fn run(id: String, profile: Option<String>, copy: bool) -> Result<()> {
    let catalog = super::load_catalog()?;
    let p = super::find_provider(&catalog, &id)?;
    let cred = super::resolve_cred(p, &id, profile.as_deref())?;

    let key = secret::store_for(&cred).get(&cred)?;
    let snippet = render::langchain(&id, p, &key)?;
    super::deliver(&snippet, copy)
}
