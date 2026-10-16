# 任务：Settings 里显示、编辑各 agent 的全局设定

2026-10-16，paddock/main 交给 paddock/dev-instructions（Claude Code，重档：opus[1m] / xhigh）。
路由：重 / 交叉审查要 / 影响面：碰要害（路由：重、要、碰要害；验证预算按用户的话不加编辑的测试，见「怎么算做完」）
类型：功能变更
依据：用户 10-16 经 cairn/main 转达（用户已同意由 paddock 做）：“是否可以在paddock的setting中显示出来各个agent的全局设定。而且可以切换到编辑。不管是claude的，还有codex和现在接入的所有的agent。”背景：全局记忆就是各工具自己的全局指令文件，cairn 不读、不写、不同步，不提供接口。主控列了 a–f 几条，用户都点了头；agy 选“只管 GEMINI.md”；看过样稿：“那就甲。开始吧”。本轮做 Settings 新的一页，只碰每家一个主文件；不碰别的配置、记忆文件，不管远端。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- 样稿 `docs/设计稿/P5-119-agent全局设定/`：README 和 `instructions.html` 里的「甲」四屏（看、改、对比、没有文件）。「乙」两屏没被选，不做。配色是 Lagoon 的近似值，实现时从主题取色；文件内容是编的示意。
- `docs/DESIGN.md` §13 里 Settings 的几条（P5-44 再设计、P5-56 立即生效、P5-94 Integrations），看现有 Settings 的规矩和画法。
- 代码：`app/src/settings.rs`（`Page`、`Page::ALL`、`label`）、`app/src/settings_view.rs`（页面怎么画、卡片、底部提示条，`settings_integrations.rs` 是一页独立成文件的现成例子）、`app/src/text_input.rs`（`wrapping(true)` 的多行编辑，Kanban 任务弹框在用）、`app/src/markdown.rs` 和 `app/src/markdown_view.rs`（Markdown 怎么画）、`app/src/agent_shell.rs` 的 `KINDS`。前后对比可以用已有的依赖 `similar`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-119-agent-instructions`，分支 `p5-119-agent-instructions`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-119-agent-instructions`。
- 动 `app/src/settings.rs`、`app/src/settings_view.rs`，新增这一页的界面文件（照 `settings_integrations.rs` 的放法）和一个读写全局文件的模块，`app/src/lib.rs` 里登记，相关测试，`docs/DESIGN.md` 和本文件。别的模块要改才做得成，先停下来报告。

## 现在是怎样（主控 10-16 查的，只看了文件在不在、多大，没读内容）
paddock 接入的 agent 是 `agent_shell::KINDS`：claude、codex、pi、omp、amp、agy。每家的主文件：

| agent | 主文件 | 环境变量 | 本机 | 依据 |
| --- | --- | --- | --- | --- |
| Claude Code | `~/.claude/CLAUDE.md` | 设了 `CLAUDE_CONFIG_DIR` 就是 `$CLAUDE_CONFIG_DIR/CLAUDE.md` | 有 | 官方约定 |
| Codex | `~/.codex/AGENTS.md` | `CODEX_HOME` | 有 | 官方约定 |
| pi | `~/.pi/agent/AGENTS.md` | `PI_CODING_AGENT_DIR` | 没有 | pi 自带 `docs/configuration.md` |
| omp | `~/.omp/agent/AGENTS.md` | 不确定，按没有处理 | 没有 | omp 程序里的说明；omp 还会读 `~/.claude/CLAUDE.md`、`~/.codex/AGENTS.md` 等别家的全局文件 |
| amp | `~/.config/amp/AGENTS.md` | 不确定，按没有处理 | 有 | 本机文件 |
| agy | `~/.gemini/GEMINI.md` | 无 | 有，空的 | agy 的更新说明；Gemini CLI 也读这个文件 |

## 要做的（照样稿甲）
1. Settings 左边在 Agents 和 Diagnostics 之间加一页 `Instructions`。
2. 页面顶上一排六家，有文件的带实心点，还没有的带空圈；点哪家下面就换成哪家。下面一张卡：主文件的路径（家目录写成 `~`）、大小、上次修改时间，右上 `Edit`；正文照 Markdown 画，只读，能滚动。卡下面一行写明生效时机（样稿原文：`Changes apply to sessions started after you save; agents already running keep what they read.`）。
3. 点 `Edit`：正文换成等宽的多行编辑框（原文，不渲染），底下 `Cancel` 和 `Review changes…`，没改动时 `Review changes…` 不能点。
4. `Review changes…`：前后对比（加的绿、删的红、没变的折叠成 `⋯`），底下写明存之前原文件备份到哪，`Back to editing` 和 `Save`。
5. 保存要安全（用户点过头的 b）：
   - 写之前把原文件复制一份备份，放 `~/.config/paddock/backups/`，文件名带 agent 和时间，不覆盖已有备份；备份不成功就不写。
   - 从打开（读进来）到保存，文件被别处改过（内容不同）的，顶上黄色提示 `This file changed on disk after you opened it.`，`Save` 不能点，只给 `Reload`（放弃自己的改动、读新版）和 `Copy my text`（把编辑框的全文放进剪贴板）。
   - 主文件是软链接的，改它指向的那个文件，链接本身不动。
   - 写入不能把文件写坏一半：先写同目录的临时文件再换上去，保留原文件的权限。
   - 写失败要把原因告诉用户，编辑框里的内容不丢。
6. 文件还不存在的（现在是 pi、omp）：卡里写明这家会读哪个文件、现在还没有，给 `Create file`；点了建好目录和空文件，进入编辑。
7. omp 那一栏写明它还会读别家的全局文件（样稿原文）；agy 那一栏写明 `Gemini CLI reads this file too.`
8. 界面文字用英文。颜色从主题取，沿用 Settings 现有的卡片、按钮、提示条画法。

## 主控提议、用户 10-16 点头的（已并进上面）
- a. 只显示、只编辑每家的主文件；Claude 的 `@` 引用、auto-memory 和各家自带的记忆文件都不读不碰。
- b. 安全保存（第 5 条）。
- c. 写明生效时机（第 2 条）。
- d. 没有文件就新建（第 6 条）；omp 写明还读别家的（第 7 条）。
- f. 只管本机，不管远端。
- agy 只管 `~/.gemini/GEMINI.md`，写明和 Gemini CLI 共用。

## 怎么算做完
- 用户原话：“是否可以在paddock的setting中显示出来各个agent的全局设定。而且可以切换到编辑。不管是claude的，还有codex和现在接入的所有的agent。”——上面「要做的」逐条达到，样子和样稿甲一致。
- 用户对测试的原话：“不要作编辑的测试，我理解这是一个显示然后可以编辑的设定。测试的时候不要加编辑的测试，但是要有可以编辑的功能。”所以：
  - 不写会去保存、备份、新建文件的测试；编辑和保存功能照样做出来。
  - 只加显示这一侧的测试，用临时目录和合成文件：每家主文件路径怎么算（含上面的环境变量）、文件不存在、软链接解析到哪个文件、读出内容和大小。照 `app/tests/` 里现成测试的写法。
- 验证只做这些：在 `app/` 下带上面的 `CARGO_TARGET_DIR` 跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，再跑 `git diff --check`，各一次。觉得不够，在回复里说，不要自己加。
- 不起窗口、不截图。样子留给用户实际看。

## 不要做
- **不读、不改、不建真实的全局文件**：本机的 `~/.claude/CLAUDE.md`、`~/.codex/AGENTS.md`、`~/.pi`、`~/.omp`、`~/.config/amp`、`~/.gemini` 下的任何文件都不打开、不 `cat`、不写。测试一律用临时目录；需要知道路径时以本文件的表为准。
- 不读、不碰各家的记忆、会话、凭证、设置文件（`~/.claude/projects`、`~/.codex/memories`、`auth.json`、`settings.json` 这类）。
- 不做样稿乙（Agents 页一节、弹框）；不做远端；不做 `@` 引用的展开；不做多文件（agy 其余三处、pi 的 `AGENTS.override.md` 等）。
- 不调 `corral start`／`stop`／`send`／`keys`；`corral ls` 里的都是用户的 agent，不碰。
- 不加依赖；不用 Python；不依赖 ratatui、crossterm；不起 paddock 窗口；不用脚本模拟按键或鼠标；不碰别的仓库。
- 命令里不用 `rm`，不用 `sh -c` 包长命令，前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。
- 不照抄 Zed 的 `terminal`、`ui` 等 GPL 代码；GPUI 本身的源码可以读来确认接口。
- 遇到要改本文件没列的模块才做得成、现有多行编辑框撑不住几 KB 的文本、或者 GPUI 画不出样稿的样子，停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-119-agent-instructions` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 做了什么：Settings 在 Agents 和 Diagnostics 之间加了一页 `Instructions`（图标 Document），照样稿甲做了看、改、对比、没有文件四屏。新模块 `app/src/instructions.rs`：六家主文件的路径（`CLAUDE_CONFIG_DIR`、`CODEX_HOME`、`PI_CODING_AGENT_DIR`，设了但为空当没设）、读（内容、大小、修改时间，读不出的写原因）、顺着软链接找到最后指向的文件、先备份再写（备份只新建、重名加 `-2`…，备份失败不写）、写同目录临时文件后改名换上去并保留原权限、新建空文件、按行对比和大小／日期的说法。新界面文件 `app/src/settings_instructions.rs`，照 `settings_integrations.rs` 挂在 `settings_view.rs` 下。`settings.rs` 加 `Page::Instructions`；`settings_view.rs` 挂上这一页，Settings 回到前台时重读，这一页不放进滚动列表，卡片占满剩下的高度。`search.rs` 加了这一页的搜索说明 `CLAUDE.md, AGENTS.md, GEMINI.md`，测试里页数 4 改成 5（主控 10-16 同意）。`docs/DESIGN.md` §13 加了 P5-119 一条。
- 验证了什么：在 `app/` 下带本任务的 `CARGO_TARGET_DIR` 跑 `cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets`（库 538 个，新增 `app/tests/instructions.rs` 5 个，全过）、`cargo fmt --check`，再跑 `git diff --check`，各一次，都一次通过。新测试只测看的一侧，用临时目录和编的文件：各家路径和环境变量、文件不存在（读了也不建东西）、相对／绝对／连环软链接和指向还没有的文件、读出的内容和大小（含空文件、不是 UTF-8 的）。没有读、写、建任何真实的全局文件；没起窗口、没截图，样子留给用户实际看。
- 拿主意的地方：
  - 编辑中顶上其余几家变淡、点不动，免得切走丢了改动；在别的 Settings 页之间切换，编辑状态还留着。
  - 备份名 `<agent>-<文件名>.<本地时间 YYYYMMDD-HHMMSS>`（几家都叫 AGENTS.md，所以带上 agent）；进对比时定名、显示，存时用这个名字，被占了才加后缀，消息条写实际的备份位置。空文件照样备份。
  - 文件变没变按内容比，在进对比、Settings 回到前台、按 `Save` 时各比一次；编辑时就发现变了的，黄条在编辑屏也出（不只在对比屏）。`Reload` 读新版后接着编辑；文件已经没了就回到看。
  - 卡头是软链接的，在路径后面淡色写 `→ <指向的文件>`，让人知道实际写的是哪个文件。
  - 卡下面那句：看的时候写 `<名字> reads this file when a session starts.`，omp、agy 接各自那句，最后是生效时机；编辑中和没有文件时只留生效时机（照样稿）。
  - 大小按 1000 算（和 Finder 一样），空文件写 `Empty`；修改时间只写日期，不是今年的带年份。
  - 不是 UTF-8 的文件只说原因、不给 `Edit`，免得按有损的文字写回去把文件弄坏。
  - 底部消息条出现时，这一页底下留出位置，不盖住卡片的 `Save` 等按钮。
- 没做的事：
  - 照用户的话，没写编辑、对比、备份、保存、新建的测试，这些只靠代码本身，没有实际点过。
  - 编辑框不会自动滚到光标处（和 Kanban 任务弹框的正文一样）；正在编辑时关掉 Settings 窗口，改动不提示就丢了（Settings 本来就没有关窗确认）。
  - 样稿乙、远端、`@` 引用展开、每家的其余文件，按任务都没做。
