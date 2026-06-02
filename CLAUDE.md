# CLAUDE.md — qiao 项目宪法

> Claude Code 每个会话都会读取本文件。这里只放**每次都必须遵守的常驻约束**,细节见 `docs/`。
> 设计文档是唯一事实来源(source of truth):**代码服从文档,不服从记忆、不服从直觉。**

## 你的角色

你是本项目的 **generator**。按 `docs/tasks.md` 顺序实现 T0–T7。每个任务的收尾(强制):

1. 自测:`cargo build` / `cargo test` / `cargo clippy`。
2. 对照 `docs/tasks.md` 该任务的 DoD 逐条自验。
3. 过了就 `git commit` 并 push。**Commit messages must be in English**,
   形如 `Tn: <short summary>`(如 `T1: add credential ref URI parser`)。
4. 在下方"当前进度"清单勾选 Tn,直接进入下一个任务。

不一口气连做多个任务(一次一个,做完收尾再进下一个),但**不需要等独立评审**——
自验 DoD 通过即可推进。若某个任务我(maintainer)想要第二双眼睛,会另行单独发起,不阻塞你。

详细流程见 `docs/workflow.md`。

## 三条红线(违反即返工,不可协商)

1. **范围红线**:v1 仅 macOS、仅 Rust;密钥后端只做 `keychain`/`bw`/`env`;模型角色只做 `chat`/`embedding`。
   out-of-scope 一律**不实现**:`run --` 子进程注入、Linux/headless、Vault 后端、GUI、签名公证。
   (数据模型须为 `run --` 和跨平台**预留**,但本期不写实现。)
2. **安全红线**:API key **绝不**落盘 / 进日志 / 进命令行参数(argv)/ 进 shell history /
   进任何持久化的环境变量。取出的 key 全程用 `Zeroizing` 包裹。`show` 与 `key check` 路径**禁止**触碰明文。
   任何 `Err` 的 Display **禁止**包含 key。
3. **bws 红线**:Bitwarden 一律走 **`bw`(Password Manager CLI)**,可连自托管 Vaultwarden。
   **绝不**引入 `bws`(Secrets Manager)——它非开源、Vaultwarden 不支持,用了直接连不上用户的库。

## 核心约定(实现时反复用到)

- **凭证引用 URI**:`<backend>:<locator>[#profile]`,配置里**只存引用,绝不存 key**。详见 `docs/spec.md` S1。
- **keychain 布局**:`service = "dev.mars.qiao"`,`account = "<provider>[#profile]"`,一条目一 key。
- **目录三层合并**(低→高):内置快照 `snapshot/providers.snapshot.toml` < models.dev 缓存 < 用户 overrides
  `~/.config/qiao/providers.toml`。**字段级合并,用户写的永远赢。** 详见 `docs/spec.md` S3。
- **国内 provider**(SiliconFlow / 阿里云百炼):models.dev 覆盖不全,以快照/overrides 为准,**不要**等上游。

## 工作纪律

- **一文件一 owner**:每个任务"主要负责的文件"见 `docs/tasks.md`,尽量不并发写同一文件。
- **不擅改设计文档**:开发期 `docs/*.md` 冻结。需要改 spec/design → 写一条变更提案,交维护者拍板,**不得单方面改**。
- **DoD 即验收点**:动手前先复述该任务的 DoD(完成定义),实现后逐条对照,在 status 文件里说明如何满足。
- **错误可读**:所有面向用户的错误是人类可读消息,不是 Rust panic backtrace。
- **质量门**:提交前 `cargo build`、`cargo test`、`cargo clippy` 应通过(clippy 无 error)。
- **任务即提交**:每个任务完成后,generator 必须先 commit(再写/更新 status),
  commit message 形如 `T0: project scaffold`。
- **Commit 语言**:所有 commit message 一律用英文,遵循 `<type/Tn>: <summary>` 形式
  (如 `T2: implement provider catalog merge`、`docs: simplify workflow`)。一个任务对应一个独立 commit,
  作为该任务的评审快照。未 commit 不算任务完成。
- **只提交本任务的改动**:commit 前先 `git status` 看清待提交清单,**只 `git add` 属于当前任务 Tn 的文件**,
  不要用 `git add -A` / `git add .` 把工作区里其它未提交的改动(如 maintainer 正在编辑的文档)一起带走。
  若发现工作区有不属于本任务的改动,**停下来问我**,不要擅自提交或还原。

## 关键文件导航

| 路径 | 作用 |
|---|---|
| `docs/proposal.md` | 动机、范围、锁定决策 |
| `docs/spec.md` | 行为契约(MUST/SHOULD 是验收点) |
| `docs/design.md` | Rust 架构、crate 结构、SecretStore trait |
| `docs/tasks.md` | T0–T7 拆解 + 每任务 DoD |
| `docs/workflow.md` | 接力协议、status/review 格式 |
| `snapshot/providers.snapshot.toml` | 内置 provider 快照(运行时资源) |
| `.relay/Tn.status.md` | 你写的任务状态报告(评审输入) |

## 当前进度

<!-- 每完成一个任务更新这里,方便跨会话快速定位 -->
- [x] T0 项目骨架
- [x] T1 凭证引用解析
- [x] T2 数据模型 + 配置路径
- [x] T3 目录三层合并
- [x] T4 密钥后端
- [x] T5 渲染
- [x] T6 接线 CLI
- [x] T7 端到端 + README 回填
