# llmkeys

> 多家 LLM provider 的密钥与配置管家。从 keychain / Bitwarden 取出 API key,
> 一键生成可直接粘贴的 `.env` 片段或 LangChain 代码片段——免去每次 google provider、
> 翻文档找 base_url、再去后台找 key 的重复劳动。

**站在巨人肩上,不重复造轮子**:provider/模型目录复用 [models.dev](https://models.dev),
密钥存储复用操作系统 keychain 与你已有的 Bitwarden/Vaultwarden,llmkeys 只写很薄的整合层。

> ✅ **当前状态:v1 已实现(仅 macOS)。**

---

## 它解决什么

日常开发里频繁切换不同 provider / 模型做对比和测试,痛点有三:

1. **死知识反复查**——每家的 base_url、模型 ID 命名、embedding 模型名,每次都要重新搜。
2. **密钥散落不安全**——API key 记在明文笔记里,有泄露和丢失风险。
3. **切换繁琐**——想用某个模型就要手动拼一整套配置。

llmkeys 把这些收敛成几条命令:列 provider、看配置、取 key 拼出 `.env` 或代码片段。

## 核心原则:机密与配置分家

| 类别 | 内容 | 存放 | 是否落盘 |
|---|---|---|---|
| 机密 | API key | keychain / `bw` | 否(配置里只存**引用**) |
| 非机密配置 | base_url、模型 ID、embedding、env 变量名 | 配置目录(快照 + models.dev + 覆盖) | 是(明文,可提交/分享) |

llmkeys 自身**不持有、不落盘**任何密钥;取出的 key 全程用 `Zeroizing` 包裹,离开作用域即擦除。

## 三个必懂概念

**① 凭证引用 URI**(配置里只存引用,绝不存 key):

```
<backend>:<locator>[#profile]
keychain:openrouter            # macOS 钥匙串,默认账号
keychain:openrouter#work       # 多账号用 #profile 区分
bw:item/OpenRouter API Key     # Bitwarden 按条目名
bw:id/2a16-445b-...            # Bitwarden 按条目 id(改名不受影响,更稳)
env:OPENROUTER_API_KEY         # 环境变量兜底
```

**② keychain 布局**:`service = "dev.mars.llmkeys"`,`account = "<provider>[#profile]"`,一个条目一个 key。

**③ 配置三层合并**(低 → 高):**内置快照 < models.dev 缓存 < 你的覆盖**(`~/.config/llmkeys/providers.toml`)。
**字段级合并、你写的永远赢**——只覆盖想改的字段,其余沿用低层。

> 注:Bitwarden 一律走 **`bw`(Password Manager CLI)**,可连自托管 Vaultwarden;
> **不用 `bws`(Secrets Manager)**——它非开源、Vaultwarden 不支持。

---

## 安装

仅 macOS(v1)。**推荐用 Homebrew**——预编译二进制,无需 Rust 工具链;brew 会自动去掉隔离属性,不触发 Gatekeeper。

### 推荐:Homebrew

```sh
brew install csthink/tap/llmkeys
llmkeys --version        # 验证
```

> 等价于 `brew tap csthink/tap && brew install llmkeys`;之后升级用 `brew upgrade llmkeys`。

### 备选:从源码构建

需要 **Rust 工具链**(`rustup`,stable)。没装过:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # 官方一键装 rustup(含 cargo)
source "$HOME/.cargo/env"                                        # 让当前终端立刻生效
cargo --version                                                  # 验证:应打印版本号
```

> 装过仍报 `cargo: command not found`,多半是终端在安装前就开着了——
> `source "$HOME/.cargo/env"` 或新开终端即可。

装成单二进制(落到 `~/.cargo/bin/llmkeys`):

```sh
git clone https://github.com/csthink/llmkeys.git
cd llmkeys
cargo install --path .
llmkeys --help        # 验证
```

> 只想本地构建不安装:`cargo build --release`,产物在 `target/release/llmkeys`。

### 可选:Bitwarden CLI(仅当你用 `bw` 后端取 key 时)

[Bitwarden CLI `bw`](https://bitwarden.com/help/cli/):

```sh
brew install bitwarden-cli      # 或 npm install -g @bitwarden/cli
bw --version                    # 验证(注意是 bw,不是 bws)
```

### 卸载 / 切换安装方式

两种装法落到不同位置(brew → `/opt/homebrew/bin`,源码 → `~/.cargo/bin`),**互不覆盖**;
同时装会按 PATH 顺序决定 `llmkeys` 指向谁(`type -a llmkeys` 可看全部)。切换前先卸旧的,免得混。

**卸载 Homebrew 版:**

```sh
brew uninstall llmkeys
brew untap csthink/tap      # 可选:不再关注更新就移除 tap
```

**卸载源码(cargo)版:**

```sh
cargo uninstall llmkeys     # 移除 ~/.cargo/bin/llmkeys
```

**从源码切到 Homebrew**(常见场景):

```sh
cargo uninstall llmkeys                 # 先卸源码版
brew install csthink/tap/llmkeys        # 再装 brew 版
```

> 卸载只动二进制。你的**配置**(`~/.config/llmkeys/`)、**models.dev 缓存**(`~/.cache/llmkeys/`)
> 和 **keychain 里的 key**(`service = dev.mars.llmkeys`)都不受影响,换装法不会丢。
> 要彻底清掉配置/缓存:`rm -rf ~/.config/llmkeys ~/.cache/llmkeys`;
> keychain key 用 [`security delete`](#管理-keychain-里的-key)。

---

## 走一遍:keychain 路线(默认)

最简单的happy path,从看 provider 到取出 `.env`,全程不碰 Bitwarden。

### 1. 列出所有 provider

合并三层后的清单(内置已预置 openrouter / siliconflow / aliyun_bailian / deepseek):

```sh
llmkeys list
```

### 2. 看某家的完整配置

key 只显示为**引用**,绝不显示明文:

```sh
llmkeys show openrouter
```

```text
openrouter
  display_name    : OpenRouter
  base_url        : https://openrouter.ai/api/v1
  key_ref         : keychain:openrouter   (引用,不显示明文)
  env_prefix      : OPENROUTER
  models.chat     : openai/gpt-5.5
  models.embedding: baai/bge-m3
```

### 3. 存入 API key(写进 keychain)

交互式粘贴,**不经命令行参数、不进 shell history、输入不回显**:

```sh
llmkeys key set openrouter
# 粘贴 keychain:openrouter 的 API key(输入不回显,回车确认):
```

### 4. 校验存没存上

只回 `yes` / `no`,不打印明文:

```sh
llmkeys key check openrouter      # → yes
```

### 5. 取出配置,拼成 `.env` 片段

key 从 keychain 取出,直接可粘进项目的 `.env`:

```sh
llmkeys env openrouter
```

```text
OPENROUTER_API_KEY=sk-...
OPENROUTER_BASE_URL=https://openrouter.ai/api/v1
OPENROUTER_MODEL=openai/gpt-5.5
OPENROUTER_EMBEDDING_MODEL=baai/bge-m3
```

> 变量名 = `env_prefix` + 固定后缀(`_API_KEY` / `_BASE_URL` / `_MODEL` / `_EMBEDDING_MODEL`);
> 缺某模型角色就省略对应行(如 deepseek 无 embedding,则无 `_EMBEDDING_MODEL` 行)。

送到剪贴板而非打印:

```sh
llmkeys env openrouter --copy
```

### 6. 或拼成 LangChain 代码片段

```sh
llmkeys code openrouter
```

```python
from langchain_openai import ChatOpenAI
llm = ChatOpenAI(
    base_url="https://openrouter.ai/api/v1",
    model="openai/gpt-5.5",
    api_key="sk-...",
)
```

---

## 改用 Bitwarden / Vaultwarden 取 key

把某家的 key 放在你已有的 Bitwarden vault 里(自托管 Vaultwarden 也可),llmkeys 调 `bw` 现取。

### 1. 在 Bitwarden 里建好条目

必须是 **Login** 类型(llmkeys 调 `bw get password`,只有 Login 有密码字段):
名称随意(如 `DEEPSEEK_API_KEY`),把 API key 填进 **password 字段**,其余留空,然后同步。

### 2. 登录并解锁 bw CLI

llmkeys 以**非交互**方式调用 `bw`,所以必须先在你的 shell 里解锁、导出会话:

```sh
bw config server https://your-vaultwarden.example.com   # 自托管才需要;官方云跳过
bw login                                                # 首次登录
export BW_SESSION="$(bw unlock --raw)"                  # 解锁并导出会话(每个新终端都要)
bw sync                                                 # 拉最新条目
```

### 3. 拿到条目 id(比按名引用更稳)

按你存的条目名搜出 id,改名也不受影响:

```sh
bw list items --search DEEPSEEK_API_KEY     # 从输出里取 "id" 字段
```

### 4. 把 key_ref 指到 bw(写进覆盖文件)

编辑 `~/.config/llmkeys/providers.toml`:

```toml
[providers.deepseek]
key_ref = "bw:id/86f99441-86ee-4632-8fda-21816a94fce2"   # 用上一步的 id
# 或按条目名:key_ref = "bw:item/DEEPSEEK_API_KEY"
```

> locator **只支持** `item/<名>` 或 `id/<id>`,别的前缀(如 `bw:llm/...`)会报错。

### 5. 端到端验证

```sh
llmkeys show deepseek      # key_ref 应显示 bw:id/...(引用,不显示明文)
llmkeys env deepseek
```

```text
DEEPSEEK_API_KEY=sk-...
DEEPSEEK_BASE_URL=https://api.deepseek.com/v1
DEEPSEEK_MODEL=deepseek-v4-pro
```

### bw 失败提示一览

所有失败都是人类可读消息(无 panic、不含明文 key):

| 场景 | 触发方式 | llmkeys 提示 |
|---|---|---|
| 已锁定 | 没 `export BW_SESSION` 就取 key | `Bitwarden 已锁定:请先 bw unlock …` |
| 未登录 | `bw logout` 后取 key | `未登录 Bitwarden:请先 bw login …` |
| 条目不存在 | `key_ref` 指向不存在的名字/id | `Bitwarden 中未找到对应条目` |
| 条目无密码 | 条目不是 Login / password 字段空 | `bw 返回了空密码:该条目可能没有 password 字段 …` |
| locator 写错 | 如 `bw:llm/deepseek` | `未知的 bw 定位类型 llm:只支持 item / id` |

> `llmkeys key set/check` **只管 keychain**,不写、不验 bw(v1 不通过 llmkeys 向 bw 写入);
> bw 的取值直接用 `llmkeys env` / `llmkeys code` 验。

---

## 多账号(`#profile`)

同一家多套 key(个人号 / 工作号)用 `#profile` 区分:

```sh
llmkeys key set openrouter#work          # 存工作号的 key
llmkeys key check openrouter#work
llmkeys env openrouter --profile work    # 取工作号
```

keychain 里对应 `account = "openrouter#work"`;bw 引用写 `bw:item/...#work`。

---

## 自定义 / 补全 provider(本地覆盖)

在 `~/.config/llmkeys/providers.toml` 写覆盖(三层合并、字段级、你写的永远赢;**只存非机密配置,绝不写 key**):

```toml
# 只改 base_url(如走自建代理),其余字段仍用快照
[providers.openrouter]
base_url = "https://my-proxy.local/v1"

# 新增一家 provider
[providers.mycorp]
display_name = "MyCorp"
base_url     = "https://api.mycorp.com/v1"
key_ref      = "keychain:mycorp"
env_prefix   = "MYCORP"
  [providers.mycorp.models]
  chat      = "mycorp-large"
  embedding = "mycorp-embed"
```

刷新 models.dev 缓存(失败自动保留旧缓存):

```sh
llmkeys refresh
```

> 国内 provider(SiliconFlow / 阿里云百炼)models.dev 收录不全,以**快照 / 你的覆盖**为准,不等上游。

---

## 管理 keychain 里的 key

llmkeys 只有 `key set` / `key check`,**没有列出 / 删除子命令**——用 macOS 自带的 `security`
(条目固定 `service = "dev.mars.llmkeys"`,`account = "<provider>[#profile]"`)。

**列出 llmkeys 存了哪些 key**(只读元数据,不取明文、不弹密码框):

```sh
security dump-keychain 2>/dev/null | awk '
  /"acct"<blob>=/ { a=$0; sub(/.*"acct"<blob>="/,"",a); sub(/".*/,"",a); acct=a }
  /"svce"<blob>=/ { s=$0; sub(/.*"svce"<blob>="/,"",s); sub(/".*/,"",s);
                    if (s=="dev.mars.llmkeys") print acct }'
```

**确认单个是否存在**(退出码 0=存在):

```sh
security find-generic-password -s "dev.mars.llmkeys" -a "openrouter" >/dev/null 2>&1 && echo yes || echo no
```

**删除某个 key**(不可逆;删前确认别处有备份):

```sh
security delete-generic-password -s "dev.mars.llmkeys" -a "openrouter"        # 默认 profile
security delete-generic-password -s "dev.mars.llmkeys" -a "openrouter#work"   # 指定 profile
```

> **把某家从 keychain 整体迁到 bw**:① 在 Bitwarden 建好条目 → ② 改 `key_ref` 为 `bw:id/<id>`
> → ③ `llmkeys env <id>` 验证能取到 → ④ **确认无误后**再用上面的 `security delete` 删掉 keychain 旧条目。
> 顺序别反,否则可能丢 key。

---

## 命令一览

| 命令 | 作用 |
|---|---|
| `llmkeys list` | 列出合并后的所有 provider(名 + base_url) |
| `llmkeys show <id>` | 展示某 provider 配置(key 为引用形式,不显示明文) |
| `llmkeys key set <id[#profile]>` | 交互式粘贴 key,写入 **keychain** |
| `llmkeys key check <id[#profile]>` | 校验 keychain 里有没有该 key(yes/no) |
| `llmkeys env <id> [--profile p] [--copy]` | 输出 `.env` 片段 |
| `llmkeys code <id> [--profile p] [--copy]` | 输出 LangChain(`ChatOpenAI`)片段 |
| `llmkeys refresh` | 重新拉取 models.dev 缓存(失败保留旧缓存) |

> 删除 keychain key 用系统 `security`(见[管理 keychain 里的 key](#管理-keychain-里的-key)),llmkeys 不提供删除子命令。

---

## 技术栈与范围(v1)

- 语言:Rust(单静态二进制);平台:**仅 macOS**。
- 密钥后端:`keychain`(默认)/ `bw` / `env`。
- 目录:models.dev 拉取 + 内置快照兜底 + 用户本地覆盖。
- 模型角色:`chat` + `embedding`(schema 预留扩展)。
- 输出:`.env` 片段 + LangChain 代码片段。

**v1 不做**(数据模型为其预留):机密注入子进程(`run --`)、Linux/headless、Vault 后端、GUI、签名公证。

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

## License

[MIT](./LICENSE) © 2026 mars

开源,无营收目标,旨在解放程序员的重复劳动。
