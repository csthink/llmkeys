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

/// llmkeys — a credential and config manager for LLM providers.
#[derive(Parser)]
#[command(name = "llmkeys", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all merged providers (name + base_url summary).
    List,

    /// Show a provider's full config (key shown as a reference, never in plaintext).
    Show {
        /// Provider id, e.g. openrouter.
        id: String,
    },

    /// Output a .env snippet (var names built from env_prefix).
    Env {
        /// Provider id.
        id: String,
        /// Select a profile (multiple accounts).
        #[arg(long)]
        profile: Option<String>,
        /// Send the output to the clipboard.
        #[arg(long)]
        copy: bool,
    },

    /// Output a LangChain code snippet (OpenAI-compatible).
    Code {
        /// Provider id.
        id: String,
        /// Select a profile (multiple accounts).
        #[arg(long)]
        profile: Option<String>,
        /// Send the output to the clipboard.
        #[arg(long)]
        copy: bool,
    },

    /// Manage keys in the keychain.
    Key {
        #[command(subcommand)]
        action: KeyAction,
    },

    /// Re-fetch models.dev and update the cache (keeps the old cache on failure).
    Refresh,
}

#[derive(Subcommand)]
enum KeyAction {
    /// Interactively prompt to paste a key, write it into the keychain (not via argv/history).
    Set {
        /// Target `<id[#profile]>`, e.g. openrouter or openrouter#work.
        target: String,
    },

    /// Check whether the key can be fetched, prints only yes/no (never the key).
    Check {
        /// Target `<id[#profile]>`.
        target: String,
    },
}

fn main() {
    if let Err(e) = run() {
        // 人类可读错误链(`{:#}` 展开 cause,不打印 Debug backtrace);Err 永不含明文 key。
        eprintln!("Error: {e:#}");
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
