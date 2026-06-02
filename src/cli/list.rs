//! `llmkeys list`:列出合并后的所有 provider(名 + base_url 摘要),并提示数据来源(S4/S6)。

use anyhow::Result;

pub fn run() -> Result<()> {
    let catalog = super::load_catalog()?;
    println!("{}", super::data_source_line());

    if catalog.providers.is_empty() {
        println!("(无 provider)");
        return Ok(());
    }

    let id_width = catalog.providers.keys().map(String::len).max().unwrap_or(0);
    for (id, p) in &catalog.providers {
        let name = p.display_name.as_deref().unwrap_or("-");
        let base_url = p.base_url.as_deref().unwrap_or("(无 base_url)");
        println!("  {id:id_width$}  {name}  {base_url}");
    }
    Ok(())
}
