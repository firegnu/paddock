# 任务：远程 New Agent 对话框里自己配 agent 的命令

2026-10-17，paddock/main 交给 paddock/dev-remote-command（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：拿不准、拿不准、拿不准，档位倾向常规；主控按规则定：对话框加一行和本机状态文件，沿用 P5-124 的远端起法和引号做法，不改用户配置、不动已在跑的 agent）
类型：功能变更
依据：用户 10-17 试 P5-124 后：“我觉得是不是可以诶用户提供一个自己配置远程agent的方式（remote new agent那个对哈框中）。这个你考虑一下”；主控给了三种放法，用户选“Command 行 + Custom…胶囊”，顺序选“先修 bug 再做自定义”（P5-124b 已合并）。看过样稿（画布「P5-124 远程开 agent 样稿」的 F、G，源文件在 `docs/设计稿/P5-124-远程开agent/`）和主控的六条提议：“可以。按照设计做吧，但是要精致”。
依赖：P5-124b
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
优先：高
你是被委派的 agent：照本文件做，不要再开别的 agent。

派出的理由：对话框的新一行、新胶囊和状态文件，界面实现按分工给 Claude Code。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部。
- 样稿：`docs/设计稿/P5-124-远程开agent/README.md`，尤其 `F-Command.dc.html`、`G-Custom.dc.html`；`C-Dialog`、`D-States` 是现在的样子。
- `docs/DESIGN.md` §13 P5-124 那条（含 P5-124b 的交互登录 shell）。
- 代码：`app/src/remote_agent_view.rs`（对话框）、`app/src/remote_start.rs`（`Request`、`login`、`look`、`RecentDirs`）、`app/tests/remote_start.rs`；本机 New Agent 的 Command 怎么拆词：`app/src/new_agent.rs` 的 `Form::args`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-124c-remote-command`，分支 `p5-124c-remote-command`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-124c-remote-command`（已从 `.target/main` 克隆）。
- 动 `app/src/` 下要动的文件、相关测试、`docs/DESIGN.md` 和本文件。不加依赖。

## 要做的（照样稿 F、G；颜色从主题取，样稿色值只是示意）
1. **Command 行**：在 Agent 和 Directory 之间加一行 `Command`，显示实际要跑的命令。选探测到的那种时预填它的命令（`Tool::command()`，如 `pi`、`codex --yolo`），可以直接改；改过（和默认不同）时下面写 `Runs through your login shell on <显示名>. Edited, kept for <显示名> ·` 加一个 `Reset to <种类>` 链接，点了回到默认；没改过只写前一句。
2. **`+ Custom…`**：Agent 那排最后加一颗 `+ Custom…`。选它时 Command 是空的，占位字 `amp, or ~/bin/my-agent --flag`，下面一句写起成功后会记成哪颗胶囊（样稿 G 的原文）。Command 空着时 Start 不能点。Agent 那排下面那句改成 `Found on <显示名>, and the ones you added.`。
3. **记住**：只在 corral 答起成功以后才记，按机器分开（键用机器的 `name`），存在本机状态文件里（和最近目录放一处、互不干扰），不写 `config.toml`。
   - 探测到的那种改过的命令记在那一种上，下次选它预填改过的；Reset 回默认后也记下（等于删掉改过的）。
   - 自定义的变成一颗胶囊，排在探测到的后面、`+ Custom…` 前面；名字取命令第一个词的最后一段（`~/bin/my-agent --fast` → `my-agent`），同名的后来者替换先来的；图标用虚线框的通用图标（样稿 F），第一个词正好是已知种类（如 `amp`）时用那一种的图标。悬停出 `×`，点了从这台机器的记录里删掉（不用确认）；删的正是选中的那颗时，选回第一颗。
   - 自定义的不去远端探测，一直列着；探测到的照 P5-124/P5-124b 每次打开现查。
4. **起法**：Command 照 shell 写法拆词（同本机 New Agent 的 Command，可以带引号），拆不开（引号没配对）时在 Command 下面当场提示；拆出的每个词再照现有的 `command_line`/`quote` 引起来，经 P5-124b 的交互登录 shell 跑 `corral start … -- <这些词>`。名字留空时用 `<目录末段>/<胶囊名或种类>`。起不来时 corral 的错照样显示在框里。
5. amp、agy 的特殊处理（插件、第一句话）不做：自己填 `amp` 能起，第一句话、插件提示照旧没有。

### 精致（用户：“但是要精致”，和 P5-124 同一套标准）
1. **对齐和尺寸**：Command 和其他三行标签列等宽、输入框左缘对齐、行距一致；胶囊多到一行放不下时整齐换行，不撑宽对话框；尺寸字号走现有界面缩放，深浅两种主题都成立。
2. **状态齐全**：胶囊（含自定义的和 `+ Custom…`）、`×`、`Reset to …` 链接都有悬停、按下、键盘焦点、不能点的样子；`×` 只在悬停那颗或它有键盘焦点时出现，出现时胶囊宽度不跳。
3. **键盘**：Tab／⇧Tab 走 Agent → Command → Directory → Name；←→ 在胶囊间换（含自定义的和 `+ Custom…`）；选 `+ Custom…` 后焦点落到 Command；⌘↩ 开始、Esc 取消；中文输入法选字时的 Return 归输入法。
4. **不跳**：换种类时 Command 的内容和下面那句跟着换，对话框宽度不变、按钮不挪；长命令在框里横向滚动或换行都行，但不撑破对话框、不被截掉看不到光标。

## 怎么算做完
- 用户原话：“我觉得是不是可以诶用户提供一个自己配置远程agent的方式（remote new agent那个对哈框中）”；选的“Command 行 + Custom…胶囊”；“可以。按照设计做吧，但是要精致”——样子和样稿 F、G 一致，并做到上面「精致」四条。
- 验证只做这些：针对这件活写测试，覆盖 Command 拆词后再引号（带空格、单双引号、`$`、`;`、反引号的命令原样到 corral，不被远端 shell 当命令执行；引号没配对时报错）、胶囊名的取法、记住的规则（起成功才记、按机器分开、改过的命令记在那一种上、Reset 删掉改过的、同名自定义替换、删掉一颗）、状态文件读写（读不到或坏了当成空的）、选 `+ Custom…` 且 Command 空时不能起。集成测试照 `tests/remote_start.rs` 用假 ssh、假登录 shell、假 corral。`git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑过一次。觉得不够，在回复里说，不要自己加。
- `docs/DESIGN.md` §13 P5-124 那条后面补一条 P5-124c，写清 Command 行、`+ Custom…`、记在哪、怎么起。

## 不要做
- 不连远程机器（包括 5090），不跑真实 ssh；不对任何机器上 `corral ls` 里的 agent 做 `stop`、`send`、`keys`、attach；不做任何断网、断线测试。
- 不改、不读用户真实的 `~/.config/paddock/config.toml` 和本机真实的状态文件，测试一律用临时目录；不起 paddock 窗口截图（样子留给用户看）。
- 集成测试不要用本机真的登录 shell（会读本机配置，把本机装的 agent 当成远端的），用假登录 shell。
- 不改本机 New Agent；不改 P5-124/P5-124b 的入口、探测、交互登录 shell、目录和名字的规则；不加 amp、agy 的插件提示和第一句话；不做 Settings 里配命令。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西；不要按项目名或路径批量杀进程。
- 遇到要改公开约定（`corral`、`ranch` 的命令或输出）才能做的，停下来报告。
- 不合并到 main，不推送。只在 `p5-124c-remote-command` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的，工具或环境和预想不一样的；没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
