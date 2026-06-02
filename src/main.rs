//! qiao CLI 入口。
//!
//! T0 只搭骨架:用 clap derive 注册全部子命令(对应 spec S4),命令体先 `todo!()`,
//! 由 T1–T6 逐步接线。`--help` / `--version` 由 clap 在分发前处理,故空壳也能列出全部子命令。
//!
//! 范围提醒(docs/proposal、CLAUDE.md 红线):v1 不注册 `run --`(D7 预留)、不碰 Linux/Vault/GUI。

use clap::{Parser, Subcommand};

// T1–T3 提供解析/数据模型/路径/目录合并;命令体由 T6 接线,故此处暂 allow(dead_code)。
#[allow(dead_code)]
mod catalog;
#[allow(dead_code)]
mod config;
#[allow(dead_code)]
mod cred_ref;
#[allow(dead_code)]
mod model;

/// qiao —— 本地 LLM provider 与密钥管家。
#[derive(Parser)]
#[command(name = "qiao", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 列出合并后的所有 provider(名 + base_url 摘要)。
    List,

    /// 展示某 provider 完整配置(key 显示为引用,绝不显示明文)。
    Show {
        /// provider id,如 openrouter。
        id: String,
    },

    /// 输出 .env 片段(按 env_prefix 拼变量名)。
    Env {
        /// provider id。
        id: String,
        /// 指定 profile(多账号)。
        #[arg(long)]
        profile: Option<String>,
        /// 把输出送到剪贴板。
        #[arg(long)]
        copy: bool,
    },

    /// 输出 LangChain 代码片段(OpenAI 兼容)。
    Code {
        /// provider id。
        id: String,
        /// 指定 profile(多账号)。
        #[arg(long)]
        profile: Option<String>,
        /// 把输出送到剪贴板。
        #[arg(long)]
        copy: bool,
    },

    /// 管理 keychain 中的密钥。
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },

    /// 重新拉取 models.dev 并更新缓存(失败时保留旧缓存)。
    Refresh,
}

#[derive(Subcommand)]
enum KeyAction {
    /// 交互式提示粘贴 key,写入 keychain(不经 argv/history)。
    Set {
        /// 目标 `<id[#profile]>`,如 openrouter 或 openrouter#work。
        target: String,
    },

    /// 校验 key 能否取出,只回 yes/no(不打印 key)。
    Check {
        /// 目标 `<id[#profile]>`。
        target: String,
    },
}

fn main() {
    let cli = Cli::parse();

    // T0 空壳:各命令体留待 T6 接线。clap 已在 parse() 阶段处理 --help/--version。
    match cli.command {
        Command::List => todo!("T6: cli::list"),
        Command::Show { .. } => todo!("T6: cli::show"),
        Command::Env { .. } => todo!("T6: cli::env"),
        Command::Code { .. } => todo!("T6: cli::code"),
        Command::Key { action } => match action {
            KeyAction::Set { .. } => todo!("T6: cli::key set"),
            KeyAction::Check { .. } => todo!("T6: cli::key check"),
        },
        Command::Refresh => todo!("T6: cli::refresh"),
    }
}
