//! `llmkeys refresh`:重新拉取 models.dev 并更新缓存;失败时保留旧缓存并提示(spec S4/S6)。

use anyhow::Result;

use crate::catalog::modelsdev;

pub fn run() -> Result<()> {
    match modelsdev::refresh() {
        Ok(file) => {
            println!("Updated the models.dev cache ({} provider entries).", file.providers.len());
            Ok(())
        }
        Err(e) => {
            // 优雅降级(S6):提示失败 + 旧缓存保留,不当作致命崩溃。
            eprintln!("Failed to fetch models.dev: {e:#}");
            eprintln!("Kept the old cache (if any); you can retry `llmkeys refresh` later.");
            Ok(())
        }
    }
}
