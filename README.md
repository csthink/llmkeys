# qiao(桥)

> 桥接多家 LLM provider 的配置中心。从 keychain / Bitwarden 取出 API key,
> 一键生成可直接粘贴的 `.env` 片段或 LangChain 代码片段——免去每次 google provider、
> 翻文档找 base_url、再去后台找 key 的重复劳动。

**站在巨人肩上,不重复造轮子**:provider/模型目录复用 [models.dev](https://models.dev),
密钥存储复用操作系统 keychain 与你已有的 Bitwarden/Vaultwarden,qiao 只写很薄的整合层。

> ✅ **当前状态:v1 已实现(macOS)。** 安装与用法见下方[安装与用法](#安装与用法)。

---

## 它解决什么

用 LangChain 写 agent 时需要频繁切换不同 provider 测试模型,痛点有三:

1. 死知识反复查——每家的 base_url、模型 ID 命名规则、embedding 模型名,每次都要重新搜。
2. 密钥散落不安全——API key 记在明文笔记里,有泄露和丢失风险。
3. 切换繁琐——想用某个模型时要手动拼一整套配置。

qiao 把这些收敛成几条命令:列出 provider、查看某家配置、取 key 拼出 `.env` 或代码片段。

## 核心原则:机密与配置分家

| 类别 | 内容 | 存放 | 是否落盘 |
|---|---|---|---|
| 机密 | API key | keychain / `bw` | 否(只存引用) |
| 非机密配置 | base_url、模型 ID、embedding、env 变量名 | 配置目录(快照 + models.dev + 覆盖) | 是(明文,可提交/分享) |

qiao 自身**不持有、不落盘**任何密钥。

## 命名约定速查

**凭证引用 URI**(配置里只存引用,绝不存 key):
```
<backend>:<locator>[#profile]
keychain:openrouter            # 默认 profile
keychain:openrouter#work       # 多账号
bw:item/OpenRouter API Key     # Bitwarden 按条目名
bw:id/2a16-445b-...            # Bitwarden 按条目 id(更稳)
env:OPENROUTER_API_KEY         # 环境变量兜底
```

**keychain 布局**:`service = "dev.mars.qiao"`,`account = "<provider>[#profile]"`,一个条目一个 key。

> 注:Bitwarden 走 **`bw`(Password Manager CLI)**,可连自托管 Vaultwarden;
> **不用 `bws`(Secrets Manager)**——它非开源、Vaultwarden 不支持。

## 设计文档

完整规格在 [`docs/`](./docs/):

| 文档 | 内容 |
|---|---|
| [proposal.md](./docs/proposal.md) | 动机、范围、已锁定决策 |
| [spec.md](./docs/spec.md) | 可测试的行为契约(命令、引用语法、schema、输出格式) |
| [design.md](./docs/design.md) | Rust 架构、crate 结构、SecretStore trait |
| [tasks.md](./docs/tasks.md) | T0–T7 实现任务拆解(单文件单 owner) |
| [workflow.md](./docs/workflow.md) | 极简开发流程:节奏、三条红线、按需评审 |

provider 内置快照:[`snapshot/providers.snapshot.toml`](./snapshot/providers.snapshot.toml)(运行时资源)。

## 开发方式

单人、小体量的自用工具,开发流程刻意保持轻:用一个 Claude Code 会话按 `docs/tasks.md`
顺序实现 T0–T7,每个任务自测、对照 DoD 自验、`git commit`(英文 message)后进入下一个。
设计文档是唯一事实来源,代码服从文档。只有在某个任务需要"第二双眼睛"时,才另开一个
独立 Claude 会话做按需评审——不搞强制评审、review 回路或自动编排(对这个规模是过度设计)。

约束以三条红线为底线(范围 / 安全 / bws),写在根目录 `CLAUDE.md`,Claude Code 每个会话自动读取。
完整流程见 [workflow.md](./docs/workflow.md)。

## 技术栈与范围(v1)

- 语言:Rust(单静态二进制),平台:**仅 macOS**
- 密钥后端:`keychain`(默认)/ `bw` / `env`
- 目录:models.dev 拉取 + 内置快照兜底 + 用户本地覆盖
- 模型角色:`chat` + `embedding`(schema 预留扩展)
- 输出:`.env` 片段 + LangChain 代码片段

**v1 不做**(数据模型为其预留):机密注入子进程(`run --`)、Linux/headless、Vault 后端、GUI、签名公证。

## 安装与用法

### 前置

- **macOS**(v1 仅支持 macOS)。
- **Rust 工具链**(`rustup`,stable)。没装过的话:

  ```sh
  # 官方一键安装 rustup(含 cargo)
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  # 让当前终端立刻生效(否则会报 `cargo: command not found`);新开终端则自动生效
  source "$HOME/.cargo/env"
  cargo --version   # 验证:应打印 cargo 版本
  ```

  > 已装过但仍报 `cargo: command not found`,多半是终端在安装前就开着了——
  > `source "$HOME/.cargo/env"` 或新开一个终端窗口即可。

- 可选:[Bitwarden CLI `bw`](https://bitwarden.com/help/cli/)(仅当你用 `bw` 后端取 key 时)。

### 安装

从源码安装(单二进制,装到 `~/.cargo/bin/qiao`):

```sh
git clone https://github.com/csthink/qiao.git
cd qiao
cargo install --path .
```

或仅构建本地二进制:`cargo build --release`(产物在 `target/release/qiao`)。

### 60 秒上手

```sh
# 1. 看有哪些 provider(合并:内置快照 + models.dev 缓存 + 你的覆盖)
qiao list

# 2. 看某家的完整配置(key 只显示为引用,绝不显示明文)
qiao show openrouter

# 3. 存入 API key —— 交互式粘贴,写进 macOS keychain
#    （不经命令行参数、不进 shell history;输入不回显）
qiao key set openrouter

# 4. 校验存没存上(只回 yes/no)
qiao key check openrouter

# 5. 取出配置,拼成可直接用的 .env 片段(key 从 keychain 取)
qiao env openrouter
#   或送到剪贴板:
qiao env openrouter --copy

# 6. 同样地,拼成 LangChain 代码片段
qiao code openrouter
```

多账号用 `#profile`:`qiao key set openrouter#work`、`qiao env openrouter --profile work`。

刷新 provider 目录(从 models.dev 拉取,失败自动保留旧缓存):`qiao refresh`。

### 命令一览

| 命令 | 作用 |
|---|---|
| `qiao list` | 列出合并后的所有 provider(名 + base_url) |
| `qiao show <id>` | 展示某 provider 配置(key 为引用形式,不显示明文) |
| `qiao key set <id[#profile]>` | 交互式粘贴 key,写入 keychain |
| `qiao key check <id[#profile]>` | 校验 keychain 里有没有该 key(yes/no) |
| `qiao env <id> [--profile p] [--copy]` | 输出 `.env` 片段 |
| `qiao code <id> [--profile p] [--copy]` | 输出 LangChain(`ChatOpenAI`)片段 |
| `qiao refresh` | 重新拉取 models.dev 缓存(失败保留旧缓存) |

### 自定义 / 补全 provider(本地覆盖)

配置三层合并(低 → 高):**内置快照 < models.dev 缓存 < 你的覆盖**,**字段级合并、你写的永远赢**。
在 `~/.config/qiao/providers.toml` 写覆盖即可(只存非机密配置,**绝不写 key**):

```toml
# 改某家的 base_url(如走自建代理),其余字段仍用快照
[providers.openrouter]
base_url = "https://my-proxy.local/v1"

# 新增一家 provider
[providers.mycorp]
display_name = "MyCorp"
base_url     = "https://api.mycorp.com/v1"
key_ref      = "keychain:mycorp"
env_prefix   = "MYCORP"
  [providers.mycorp.models]
  chat = "mycorp-large"
```

> 国内 provider(SiliconFlow / 阿里云百炼)以**快照 / 你的覆盖**为准,不等上游 models.dev 收录。

### 用 Bitwarden / Vaultwarden 取 key

把某家的 `key_ref` 指到 `bw` 后端(在覆盖文件里),key 仍存在你的 Bitwarden vault:

```toml
[providers.openrouter]
key_ref = "bw:item/OpenRouter API Key"   # 按条目名;或 bw:id/<条目 id>(更稳)
```

前置:先登录并解锁 CLI——

```sh
bw config server https://your-vaultwarden.example.com   # 自托管 Vaultwarden
bw login
export BW_SESSION="$(bw unlock --raw)"
qiao env openrouter   # qiao 会调用 bw get 取 key
```

> Bitwarden 一律走 **`bw`(Password Manager CLI)**,可连自托管 Vaultwarden;
> **不用 `bws`(Secrets Manager)**——它非开源、Vaultwarden 不支持。

## License

[MIT](./LICENSE) © 2026 mars

开源,无营收目标,旨在解放程序员的重复劳动。