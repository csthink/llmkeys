//! llmkeys CLI 入口。
//!
//! clap derive 注册全部子命令(对应 spec S4),T6 把命令体接到 `cli` 模块的数据流(design D3)。
//! `--help` / `--version` 由 clap 在分发前处理。错误统一在 `main` 以人类可读形式打印(无 panic backtrace)。
//!
//! 范围提醒(docs/proposal、CLAUDE.md 红线):v1 不注册 `run --`(D7 预留)、不碰 Linux/Vault/GUI。

use clap::{Parser, Subcommand};

mod catalog;
mod cli;
mod config;
mod cred_ref;
mod model;
mod render;
mod secret;

/// llmkeys —— 本地 LLM provider 与密钥管家。
#[derive(Parser)]
#[command(name = "llmkeys", version, about, long_about = None)]
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
    if let Err(e) = run() {
        // 人类可读错误链(`{:#}` 展开 cause,不打印 Debug backtrace);Err 永不含明文 key。
        eprintln!("错误:{e:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::List => cli::list::run(),
        Command::Show { id } => cli::show::run(id),
        Command::Env { id, profile, copy } => cli::env::run(id, profile, copy),
        Command::Code { id, profile, copy } => cli::code::run(id, profile, copy),
        Command::Key { action } => match action {
            KeyAction::Set { target } => cli::key::set(target),
            KeyAction::Check { target } => cli::key::check(target),
        },
        Command::Refresh => cli::refresh::run(),
    }
}
