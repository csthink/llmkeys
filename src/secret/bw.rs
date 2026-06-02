//! bw 后端:shell out 到 Bitwarden Password Manager CLI(`bw`,design D3)。
//!
//! **红线**:一律 `bw`(Password Manager CLI,可连自托管 Vaultwarden),**绝不** `bws`。
//!
//! locator 形态:`item/<名>` 或 `id/<id>`,二者都映射到 `bw get password <值>`。key 经
//! `bw` 的 **stdout** 返回(不进 argv);失败信息只取 stderr(不含明文 key)。

use std::process::Command;

use anyhow::{anyhow, Context, Result};
use zeroize::{Zeroize, Zeroizing};

use super::{Secret, SecretStore};
use crate::cred_ref::CredRef;

pub struct BwStore;

/// 从 locator 取出查询值。`item/<名>` 与 `id/<id>` v1 都用 `bw get password <值>`。
fn query_value(locator: &str) -> Result<&str> {
    let (kind, value) = locator
        .split_once('/')
        .ok_or_else(|| anyhow!("bw 引用的 locator 须形如 `item/<名>` 或 `id/<id>`"))?;
    match kind {
        "item" | "id" => {
            if value.is_empty() {
                Err(anyhow!("bw locator 的 `{kind}/` 之后不能为空"))
            } else {
                Ok(value)
            }
        }
        other => Err(anyhow!("未知的 bw 定位类型 `{other}`:只支持 `item` / `id`")),
    }
}

/// `bw` 非零退出的归类(只看 stderr,stderr 不含明文 key)。
#[derive(Debug, PartialEq, Eq)]
enum BwFailure {
    NotLoggedIn,
    Locked,
    NotFound,
    Other(String),
}

fn classify(stderr: &str) -> BwFailure {
    let s = stderr.to_lowercase();
    if s.contains("not logged in") {
        BwFailure::NotLoggedIn
    } else if s.contains("locked") {
        BwFailure::Locked
    } else if s.contains("not found") || s.contains("no items") {
        BwFailure::NotFound
    } else {
        BwFailure::Other(stderr.trim().to_string())
    }
}

/// 把失败归类转成可操作的人类可读错误(NotFound 由调用方按 get/exists 区别处理)。
fn actionable(failure: &BwFailure) -> anyhow::Error {
    match failure {
        BwFailure::NotLoggedIn => anyhow!(
            "未登录 Bitwarden:请先 `bw login`(自托管先 `bw config server <你的 Vaultwarden 地址>`)"
        ),
        BwFailure::Locked => {
            anyhow!("Bitwarden 已锁定:请先 `bw unlock` 并 `export BW_SESSION=<返回的会话>`")
        }
        BwFailure::NotFound => anyhow!("Bitwarden 中未找到对应条目"),
        BwFailure::Other(msg) => anyhow!("bw 调用失败:{msg}"),
    }
}

/// 执行 `bw get password <value>`。成功 → Ok(Some(secret));NotFound → Ok(None);
/// 把子进程 stdout 字节转成受控 [`Secret`]:**move** 进 String(复用同一缓冲、零拷贝),
/// 就地去掉尾部换行。明文的**首个落点**即由 `Zeroizing` 托管,不在裸 `Vec` / 中间 `String`
/// 留残影(对照 keychain.rs::get 的 `Zeroizing::new(pw)`)。非 UTF-8 则擦除回收字节后报错。
fn finish_password(stdout: Vec<u8>) -> Result<Secret> {
    let mut key = match String::from_utf8(stdout) {
        Ok(s) => Zeroizing::new(s),
        Err(e) => {
            // 把回收到的首个落点字节立即擦除,错误消息不含明文。
            let mut bytes = e.into_bytes();
            bytes.zeroize();
            return Err(anyhow!("bw 返回的内容不是合法 UTF-8"));
        }
    };
    // truncate 就地缩短,不产生新的明文拷贝;被切掉的只可能是换行符,非 key 字节。
    let end = key.trim_end_matches(['\n', '\r']).len();
    key.truncate(end);
    // 防线二:success + 空 stdout 不是合法 key(条目无 password 字段,或被吞掉的崩溃)。
    // 宁可报错也不渲染 `*_API_KEY=`(空),以免下游静默用一个不存在的 key。
    if key.is_empty() {
        return Err(anyhow!(
            "bw 返回了空密码:该条目可能没有 password 字段(API key 或存在自定义字段/备注里,v1 暂只读 password 字段)"
        ));
    }
    Ok(key)
}

/// 其它失败 → Err(可操作消息)。把 NotFound 与硬错误分开,便于 exists 复用。
fn run_get(value: &str) -> Result<Option<Secret>> {
    // `--nointeraction`:禁止 bw 在锁定/未登录时交互式提示主密码。否则因 stdin 非 TTY,
    // bw 会读不到输入而崩溃,却仍以**退出码 0 + 空 stdout** 返回,被误当成"取到空 key"。
    // 加此旗后,锁定 → exit≠0 + stderr `Vault is locked.`,交由 classify 归类成可操作错误。
    let output = Command::new("bw")
        .arg("--nointeraction")
        .arg("get")
        .arg("password")
        .arg(value)
        .output()
        .context("无法执行 `bw`:请确认已安装 Bitwarden CLI 且在 PATH 中")?;

    if output.status.success() {
        // key 只在 stdout。直接把该缓冲 move 进受控 Secret(零拷贝),首个落点即清零。
        return Ok(Some(finish_password(output.stdout)?));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    match classify(&stderr) {
        BwFailure::NotFound => Ok(None),
        other => Err(actionable(&other)),
    }
}

impl SecretStore for BwStore {
    fn get(&self, r: &CredRef) -> Result<Secret> {
        let value = query_value(&r.locator)?;
        run_get(value)?.ok_or_else(|| actionable(&BwFailure::NotFound))
    }

    fn set(&self, _r: &CredRef, _value: Secret) -> Result<()> {
        // v1 不在 bw 后端写入(design D3):避免把 key 经 argv/临时文件喂给 bw create。
        Err(anyhow!(
            "v1 不支持通过 llmkeys 写入 Bitwarden;请在 Bitwarden 客户端手动创建该条目后,用 `bw:item/<名>` 引用"
        ))
    }

    fn exists(&self, r: &CredRef) -> Result<bool> {
        let value = query_value(&r.locator)?;
        // run_get 成功返回的 Secret 在此立即丢弃,只回 bool(不打印明文)。
        Ok(run_get(value)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_item_and_id_locators() {
        assert_eq!(query_value("item/OpenRouter API Key").unwrap(), "OpenRouter API Key");
        assert_eq!(query_value("id/2a16-445b").unwrap(), "2a16-445b");
    }

    #[test]
    fn reject_bad_locators() {
        assert!(query_value("openrouter").is_err()); // 无 '/'
        assert!(query_value("vault/x").is_err()); // 未知类型
        assert!(query_value("item/").is_err()); // 空值
    }

    #[test]
    fn finish_password_moves_trims_and_wraps() {
        // 占位串,非真实 key。验证零拷贝 move + 尾换行去除。
        assert_eq!(&*finish_password(b"sk-abc123\n".to_vec()).unwrap(), "sk-abc123");
        assert_eq!(&*finish_password(b"sk-xyz\r\n".to_vec()).unwrap(), "sk-xyz");
        assert_eq!(&*finish_password(b"no-newline".to_vec()).unwrap(), "no-newline");
    }

    #[test]
    fn finish_password_rejects_empty() {
        // success + 空(或仅换行)stdout 必须报错,绝不当成空 key 渲染出 `*_API_KEY=`。
        for raw in [b"".as_slice(), b"\n", b"\r\n"] {
            let err = finish_password(raw.to_vec()).unwrap_err().to_string();
            assert!(err.contains("空密码"), "应报空密码错误,得到:{err}");
        }
    }

    #[test]
    fn classify_known_failures() {
        assert_eq!(classify("You are not logged in."), BwFailure::NotLoggedIn);
        assert_eq!(classify("Vault is locked."), BwFailure::Locked);
        assert_eq!(classify("Not found."), BwFailure::NotFound);
        match classify("some network error") {
            BwFailure::Other(m) => assert!(m.contains("network")),
            _ => panic!("应归类为 Other"),
        }
    }

    #[test]
    fn actionable_messages_are_helpful_and_keyless() {
        // 消息含可操作指引,且天然不含任何 key。
        assert!(actionable(&BwFailure::NotLoggedIn).to_string().contains("bw login"));
        assert!(actionable(&BwFailure::Locked).to_string().contains("bw unlock"));
    }

    #[test]
    fn set_is_unsupported_with_guidance() {
        let r = CredRef {
            backend: crate::cred_ref::Backend::Bw,
            locator: "item/x".into(),
            profile: None,
        };
        let err = BwStore.set(&r, Zeroizing::new("k".into())).unwrap_err();
        assert!(err.to_string().contains("手动创建"));
    }
}
