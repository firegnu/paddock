# 任务：New Agent 的模型和强度改从配置文件取，Codex 每月自动取一次

2026-10-14，paddock/main 记。
依据：用户 10-14 问“现在claude和codex中的模型还有effort是不是预先写好的？而不是实时得到的。对吧？”，让主控“调研一下”。调研（见文末「调研结论」）后用户定：“能否改为从配置文件取？另外我觉得codex一天一次还是频繁。我觉得一个月一次都可以。哈哈。但是codex娶不到要有回退到写死的fallback。配置文件codex和claude都写，取到就覆盖codex，取不到就fallback到codex的配置文件。”主控建议取到的另存缓存、不写回配置文件，加手动刷新；用户：“1: 同意你的建议 2: 不同意，那是我经常用的”（2 指 `opus[1m]`：留着）。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-100-models`，分支 `p5-100-models`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-100-models`。
- 动 `app/src/new_agent.rs`（列表从哪来）、`new_agent_view.rs`（强度随模型）、`config.rs`（新配置项）、新文件放取 Codex 目录和缓存、Settings › Agents（刷新）、测试、`docs/DESIGN.md` 和本文件。

## 要做的

### 一、三层来源
New Agent 里 Claude、Codex 的模型和强度按这个次序取，前一层没有才用后一层：
1. **Codex 取到的目录**（只对 Codex）：paddock 自己的缓存文件 `~/.local/state/paddock/codex-models.json`（和 `layout.json`、`workspaces.json` 放一起），一个月内取到的才用。
2. **配置文件** `~/.config/paddock/config.toml` 里用户写的（paddock 只读，不写回）。
3. **代码里写死的**。

### 二、配置文件怎么写
新加一节，两家都能写，没写的那家用写死的：
```toml
[agent_models.claude]
models = ["opus[1m]", "fable", "opus", "sonnet", "haiku"]
efforts = ["low", "medium", "high", "xhigh", "max"]

[agent_models.codex]
models = ["gpt-6.1-sol", "gpt-6-astra", "gpt-6-sol", "gpt-6-luna", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5"]
efforts = ["low", "medium", "high", "xhigh", "max"]
```
- 列表第一个是选这家时默认的模型；强度默认照旧 `high`（列表里没有 `high` 时用最后一个）。
- 写错（不是字符串、空列表）时照现在配置文件出错的做法报错，不悄悄忽略。
- 改了配置文件，下次打开 New Agent 生效。
- 现在不在 Settings 里给编辑界面，手改文件。

### 三、写死的那份（第 3 层）更新
- Claude：`opus[1m]`、`fable`、`opus`、`sonnet`、`haiku`；强度 low～max。`opus[1m]` 留着且排第一（用户：“那是我经常用的”），预设不动。
- Codex：照 10-14 取到的目录里给人选的 8 个，按 Codex 自己的顺序：`gpt-6.1-sol`、`gpt-6-astra`、`gpt-6-sol`、`gpt-6-luna`、`gpt-5.6-sol`、`gpt-5.6-terra`、`gpt-5.6-luna`、`gpt-5.5`；强度 low～max（现在只到 xhigh）。
- 这样 Codex 默认模型从 `gpt-6-astra` 变成 `gpt-6.1-sol`（Codex 自己排第一的）；预设 Codex builder 写明了 `gpt-6-astra`，不受影响。

### 四、Codex 每月取一次
- 打开 New Agent 面板时，缓存没有或已超过 30 天，就在后台跑 `codex debug models`，面板照常用、先显示现有的列表，取到后换上。
- 只取 `visibility` 是 `list` 的，按 `priority` 排；每个模型带它自己支持的强度（`supported_reasoning_levels`，有的有 `ultra`），存进缓存，连同取的时间。
- 用的是 Codex 取到的目录时，强度那排跟着选中的模型变：只显示这个模型支持的；换模型时原来的强度新模型也有就保留，没有就用 `high`，再没有用最后一个。
- 取失败（没装 Codex、没登录、命令没了、输出看不懂）：不报错打扰，沿用上次的缓存（超过 30 天也照用）；连缓存都没有就用配置文件、写死的。下个月再试。失败原因在 Settings 里看得到（见五）。
- 只从 paddock 自己调这一条命令，不读 Codex 自己的缓存文件。

### 五、Settings › Agents 里手动刷新
- 加一行说明 Codex 模型从哪来、什么时候取的，旁边一个 `Refresh`，点了立刻取一次。英文，大致：`Codex models: fetched Oct 14 from codex.` `Refresh`；没取到时 `Codex models: from your config (couldn't fetch: <原因>)` 之类。具体放哪、怎么写，做的时候出样稿给用户看。

## 怎么算做完
用户原话：“codex取，但是不能太频繁”“一个月一次都可以”“codex娶不到要有回退到写死的fallback”“配置文件codex和claude都写，取到就覆盖codex，取不到就fallback到codex的配置文件”“claude写死吧”。

## 用户点过头的
- 取到的 Codex 列表存在 paddock 自己的缓存文件，不写回 config.toml；加 `Refresh`。
- Codex 默认模型随 Codex 自己的排序变（现在会从 `gpt-6-astra` 变成 `gpt-6.1-sol`）。
- 用户 10-14 回：“可以。但是opus5.5 1m那个不能去掉”（`opus[1m]` 留着、排第一，见三）。

## 调研结论（10-14，主控）
- Codex 0.162.1：`codex debug models` 联网取模型目录（JSON，约 7 秒），每个模型有 `slug`、`display_name`、`description`、`visibility`、`priority`、`default_reasoning_level`、`supported_reasoning_levels`。这是 `debug` 子命令，不是 Codex 承诺不变的接口，所以要兜底。
- Claude Code 2.1.299：没有列模型的命令；`/v1/models` 只写了 API key 认证，订阅登录能不能用没写。简写 `fable`（Fable 5.1）、`opus`（Opus 5.5）、`sonnet`、`haiku` 会随 Claude Code 版本指向新模型；Opus 4.7 以后默认 1M 上下文。`--effort` 本机帮助是 low～max（文档另说有 `ultracode`，没核实，不加）。

## 完成记录（10-14，主控）

### 做了什么
- 新文件 `app/src/models.rs`：三层来源（Codex 取到的 → `[agent_models.*]` → 写死的），`codex debug models` 的解析（`visibility == "list"`、按 `priority`、各带 `supported_reasoning_levels`），缓存 `~/.local/state/paddock/codex-models.json`（写临时文件再改名），30 天一问，失败沿用上次的并记原因（`codex not found`、codex 自己 stderr 的第一行、`unexpected output`），同时只有一处在问。
- `config.rs`：`[agent_models.claude]`、`[agent_models.codex]`，各 `models`、`efforts`，空、空白、非字符串、多余键、别的种类都报错。
- `new_agent.rs`：去掉写死的 `CLAUDE_MODELS`、`CODEX_MODELS`、`EFFORTS`；表单带当前列表，选种类取第一个模型和 `high`（没有就最后一个），换模型时强度能留就留。写死的列表照三更新，Codex 默认变 `gpt-6.1-sol`；预设不动。
- `new_agent_view.rs`：打开、重开 New Agent 时读配置和缓存；满 30 天在后台问 codex，答了换上列表（已选的不动）；Settings 保存时也换。模型选单两个一行，强度一行最多六个（Codex 8 个模型、带 `ultra` 的 6 档强度，原来一行放不下）。
- Settings › Agents：照样稿在 corral 下面加 Models 一节（`settings_models.rs`），`Refresh` 立刻问一次；问着显示 `Asking codex…`、按钮变灰。

### 验证
- 先写测试看红再实现：`models.rs` 10 条（来源次序、强度随模型、配置检查、30 天、解析、缓存读写、用假 codex 脚本跑刷新成功／输出看不懂／退出码非 0／找不到程序）；`config.rs` 1 条；`new_agent.rs` 改 1 条、加 1 条。
- `cargo test --all-targets`（库内 517 条及各集成测试）全过；`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都干净。
- 本机跑了一次 `codex debug models` 核对字段：现在给人选的是 7 个（`gpt-5.5` 不在了），强度最多 6 档（low～max、ultra），解析用的字段都在。
- 没起窗口看：Settings 的 Models 一节和 New Agent 分行后的选单留给用户实际看。

### 拿主意的地方
- 缓存里问失败也记时间，所以失败后也要等一个月（或点 Refresh）才再问，不会每次打开都卡 7 秒去试。
- 选单分行是为了放下 8 个模型，样稿没画这一处。
- 已经选好的模型不在新列表里时不改它（命令里照样写），选单里只是没有亮的那格。
- 写死的 Codex 列表照任务文件写的 10-14 那 8 个，没跟着这次核对去掉 `gpt-5.5`；装好后第一次打开 New Agent 就会从 codex 取到最新的。

### 没做
- Settings 里编辑模型列表（任务写明手改文件）。
- Claude 取模型（没有可用的命令）。

## 用户看过（10-14）
- 重开看过：“我重开看了，gpt-5.5 去掉吧，然后默认设为astra high”。
- 主控照做（分支 `p5-100-astra`）：写死的 Codex 列表去掉 `gpt-5.5`，剩 7 个；Codex 默认改成 `gpt-6-astra`、`high`。不管列表来自 codex、配置文件还是写死的，只要有 `gpt-6-astra` 就默认它，没有才用第一个；列表本身仍按 Codex 的顺序排。Claude 照旧取列表第一个。测试先改成新期望看红，再实现看绿；`cargo test --all-targets`、clippy、fmt、`git diff --check` 都过。
