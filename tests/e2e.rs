//! 端到端测试(spec S4/S5,tasks T7)。
//!
//! 通过**编译出的真实二进制**走通数据流;用 **env 后端**做 keychain 的 hermetic mock
//! (env 后端即 design D3 的"CI/测试兜底"),配合临时 `XDG_CONFIG_HOME` / `XDG_CACHE_HOME`
//! 隔离配置与缓存——全程不碰真实 keychain、不打网络。
//!
//! 安全:所有用例的 key 都是占位串,绝非真实 key。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// cargo 为集成测试注入的二进制路径。
const BIN: &str = env!("CARGO_BIN_EXE_llmkeys");

/// 建一个干净的临时目录,同时充当 XDG_CONFIG_HOME 与 XDG_CACHE_HOME。
fn temp_home(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("llmkeys-e2e-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// 在临时配置目录写一份 overrides,把 openrouter 的 key_ref 指到 env 后端。
///
/// config.rs 在 `XDG_CONFIG_HOME` 后追加 `llmkeys` 子目录,故文件落在 `<home>/llmkeys/providers.toml`。
fn write_env_override(home: &Path) {
    let cfg_dir = home.join("llmkeys");
    fs::create_dir_all(&cfg_dir).unwrap();
    fs::write(
        cfg_dir.join("providers.toml"),
        "[providers.openrouter]\nkey_ref = \"env:LLMKEYS_E2E_KEY\"\n",
    )
    .unwrap();
}

/// 跑一条 llmkeys 命令,隔离到临时 home,并注入占位 key 变量。
fn run(home: &Path, args: &[&str], key: Option<&str>) -> Output {
    let mut cmd = Command::new(BIN);
    cmd.args(args)
        .env("XDG_CONFIG_HOME", home)
        .env("XDG_CACHE_HOME", home);
    if let Some(k) = key {
        cmd.env("LLMKEYS_E2E_KEY", k);
    }
    cmd.output().expect("运行 llmkeys 失败")
}

#[test]
fn env_command_renders_dotenv_from_env_backend() {
    let home = temp_home("dotenv");
    write_env_override(&home);

    let out = run(&home, &["env", "openrouter"], Some("placeholder-not-real"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // base_url / model 来自内置快照;key 来自 env 后端(keychain 的 mock)。
    assert!(stdout.contains("OPENROUTER_BASE_URL=https://openrouter.ai/api/v1"));
    assert!(stdout.contains("OPENROUTER_MODEL=openai/gpt-5.5"));
    assert!(stdout.contains("OPENROUTER_API_KEY=placeholder-not-real"));

    fs::remove_dir_all(&home).ok();
}

#[test]
fn code_command_renders_langchain_from_env_backend() {
    let home = temp_home("langchain");
    write_env_override(&home);

    let out = run(&home, &["code", "openrouter"], Some("placeholder-not-real"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("from langchain_openai import ChatOpenAI"));
    assert!(stdout.contains("base_url=\"https://openrouter.ai/api/v1\""));
    assert!(stdout.contains("model=\"openai/gpt-5.5\""));
    assert!(stdout.contains("api_key=\"placeholder-not-real\""));

    fs::remove_dir_all(&home).ok();
}

#[test]
fn env_copy_prints_snippet_and_reports_copied() {
    // 走通 --copy/arboard 路径:片段仍在 stdout,状态提示在 stderr;exit 0(剪贴板失败也优雅降级)。
    let home = temp_home("copy");
    write_env_override(&home);

    let out = run(&home, &["env", "openrouter", "--copy"], Some("placeholder-not-real"));
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // 片段仍写 stdout(剪贴板在无头环境难断言,故只验输出 + 提示)。
    assert!(stdout.contains("OPENROUTER_BASE_URL=https://openrouter.ai/api/v1"));
    assert!(stdout.contains("OPENROUTER_API_KEY=placeholder-not-real"));
    // SHOULD「已复制」提示;无头环境复制失败则给降级提示——两者都证明 --copy 路径被执行。
    assert!(
        stderr.contains("已复制到剪贴板") || stderr.contains("复制到剪贴板失败"),
        "stderr 应含复制结果提示,实际: {stderr}"
    );

    fs::remove_dir_all(&home).ok();
}

#[test]
fn list_works_offline_from_snapshot() {
    // 空临时 home:无 overrides、无 models.dev 缓存 → 全走内置快照。
    let home = temp_home("list");

    let out = run(&home, &["list"], None);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    for id in ["openrouter", "deepseek", "siliconflow", "aliyun_bailian"] {
        assert!(stdout.contains(id), "list 缺 provider: {id}");
    }
    // 国内 provider 以快照为准(PROPOSAL-001 A):siliconflow 显 .cn、deepseek 带 /v1。
    assert!(stdout.contains("https://api.siliconflow.cn/v1"));
    assert!(stdout.contains("https://api.deepseek.com/v1"));

    // 坐实离线:list 在 cache-miss 时只回退快照,绝不 live-fetch(reqwest 仅 modelsdev::fetch 用,
    // 而 fetch 只被 refresh 调,refresh 只被 `llmkeys refresh` 调)。若 list 触网拉取会写缓存——
    // 断言隔离 cache 目录里没有 modelsdev.json,以防此假设被未来改动悄悄打破。
    let cache_file = home.join("llmkeys").join("modelsdev.json");
    assert!(
        !cache_file.exists(),
        "list 不应拉取/写入 models.dev 缓存(cache-miss 应只读内置快照)"
    );

    fs::remove_dir_all(&home).ok();
}

#[test]
fn show_displays_key_ref_not_plaintext() {
    // 安全回归:即便 env 变量已设占位"密钥",show 也绝不读出/打印它,只显示引用。
    let home = temp_home("show");
    write_env_override(&home);

    let out = run(&home, &["show", "openrouter"], Some("SUPER-SECRET-PLACEHOLDER"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("env:LLMKEYS_E2E_KEY"), "应显示 key_ref 引用");
    assert!(
        !stdout.contains("SUPER-SECRET-PLACEHOLDER"),
        "show 绝不能打印明文 key"
    );

    fs::remove_dir_all(&home).ok();
}

#[test]
fn unknown_provider_errors_readably() {
    let home = temp_home("unknown");

    let out = run(&home, &["env", "does-not-exist"], None);
    assert!(!out.status.success(), "未知 provider 应以非零退出");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("未找到 provider"));
    // 人类可读消息,不是 panic backtrace。
    assert!(!stderr.contains("panicked"));

    fs::remove_dir_all(&home).ok();
}

#[test]
fn env_missing_key_gives_actionable_error() {
    // key_ref 指到一个未设置的 env 变量 → 可操作错误,且不泄露任何明文。
    let home = temp_home("missing-key");
    write_env_override(&home);

    // 不注入 LLMKEYS_E2E_KEY。
    let out = run(&home, &["env", "openrouter"], None);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("LLMKEYS_E2E_KEY"));
    assert!(stderr.contains("export"));

    fs::remove_dir_all(&home).ok();
}
