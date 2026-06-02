//! `qiao refresh`:重新拉取 models.dev 并更新缓存;失败时保留旧缓存并提示(spec S4/S6)。

use anyhow::Result;

use crate::catalog::modelsdev;

pub fn run() -> Result<()> {
    match modelsdev::refresh() {
        Ok(file) => {
            println!("已更新 models.dev 缓存({} 个 provider 条目)。", file.providers.len());
            Ok(())
        }
        Err(e) => {
            // 优雅降级(S6):提示失败 + 旧缓存保留,不当作致命崩溃。
            eprintln!("models.dev 拉取失败:{e:#}");
            eprintln!("已保留旧缓存(如有);稍后可重试 `qiao refresh`。");
            Ok(())
        }
    }
}
