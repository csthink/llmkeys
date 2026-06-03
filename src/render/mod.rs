//! 渲染:把合并后的 `Provider` + 取出的明文 key 拼成 `.env` 片段与 LangChain 代码片段(spec S5)。
//!
//! 安全:产物**含明文 key**(S5 要求),这是 key 唯一合法出现在输出里的地方(由 `env`/`code`
//! 命令打印或 `--copy`)。故渲染函数返回 [`Secret`](`Zeroizing<String>`):明文只落在这一个受控
//! 缓冲,打印/复制后离开作用域即清零,不留非清零的中间副本(延续 secret 模块的内存卫生)。
//!
//! 字段策略(spec S5 / S3):
//! - 缺某模型角色 → 省略对应行(chat→`_MODEL`、embedding→`_EMBEDDING_MODEL`)。
//! - 缺 MUST 字段(`env_prefix` / `base_url`)→ 返回**指明 provider + 字段名**的错误(S3)。

pub mod dotenv;
pub mod langchain;

pub use dotenv::render as dotenv;
pub use langchain::render as langchain;

use anyhow::{anyhow, Result};

/// 取必填字段;缺失则报指明 provider + 字段名的错误(spec S3)。子模块经 `super::require` 复用。
fn require<'a>(field: Option<&'a str>, id: &str, name: &str) -> Result<&'a str> {
    field.ok_or_else(|| anyhow!("provider {id} is missing the required field `{name}`, cannot render"))
}
