//! `llmkeys show <id>`:展示某 provider 完整配置。
//!
//! **安全**:key 一律显示为 `key_ref` 引用形式,**绝不**取出 / 显示明文(本命令不碰 secret 后端)。

use anyhow::Result;

fn field(label: &str, value: Option<&str>) {
    match value {
        Some(v) => println!("  {label:16}: {v}"),
        None => println!("  {label:16}: (未设置)"),
    }
}

pub fn run(id: String) -> Result<()> {
    let catalog = super::load_catalog()?;
    let p = super::find_provider(&catalog, &id)?;

    println!("{id}");
    field("display_name", p.display_name.as_deref());
    field("base_url", p.base_url.as_deref());
    // key_ref 原样打印(引用,不显示明文)。
    match p.key_ref.as_deref() {
        Some(kr) => println!("  {:16}: {kr}   (引用,不显示明文)", "key_ref"),
        None => println!("  {:16}: (未设置)", "key_ref"),
    }
    field("env_prefix", p.env_prefix.as_deref());
    field("naming_note", p.naming_note.as_deref());
    field("models.chat", p.models.chat());
    field("models.embedding", p.models.embedding());
    Ok(())
}
