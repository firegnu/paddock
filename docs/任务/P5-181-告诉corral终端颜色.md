# 任务：把终端主题的前景、背景色告诉 corral

2026-10-24，paddock/main 交给 paddock/dev-colors（Codex，中：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.85）、拿不准、拿不准；主控定：做错最多是 agent 启动时颜色不对，算改行为；交叉审查照用户 10-19 定由主控审）
类型：功能变更
依据：用户 10-24 转来 ranch R50 的需求，主控评估后用户定：“加第4条，现在就做，派出去吧”。起因是 10-24 用户问的“为什么 Claude Code、Codex 设成跟随终端却是深色”：后台开的 agent 启动时问终端颜色，没有界面连着就没人答，它们按深色处理。本轮只做 paddock 这边调用 corral 的公开命令，不改 ranch。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户转来的需求（ranch，原文）
- ranch R50 新增 `corral terminal-colors --fg '#rrggbb' --bg '#rrggbb'`（兼容新增，约定见 ranch `docs/调研/R50-corral回答终端颜色查询.md` §4，合并后进 CONTRACT）。paddock 要做的：
  1. 启动时、终端主题变化时（包括跟着系统切深浅），用当前主题的前景色、背景色调一次这个命令；
  2. 在远端机器开 agent 前，对那台机器的 corral 也调一次；
  3. 调用失败（旧版 corral 报 usage）就忽略，不要挡住开 agent；
  4. 可选：在设置里说明已经在跑的 Codex 不会跟着换主题，要重开。
- 用户：“评估一下这个事情。看看工作量有多大，并根据这个添加一个draft task”；看过评估后：“加第4条，现在就做，派出去吧”。

## 约定要点（ranch R50 §4；以合并后的 CONTRACT 为准）
- `corral terminal-colors --fg C --bg C` 设定、`--clear` 清除、不带参数只查；成功一行 `{"ok":true,"terminal_colors":{"fg":…,"bg":…}}`、退出码 0；`COLOR` 是 `#rrggbb`；格式不对、只给一个报 `usage`、退出码 1，原设置不变。
- 设定是本机全局的，从下一次 `corral start` 起对所有人开的 agent 生效，已在跑的 agent 不变；有界面 attach 着时照旧由界面回答。
- 10-24 主控查过：本机装的 corral 还不认这个命令（报 `usage`），ranch R50b 正在实现。所以现在调用会失败，照第 3 条忽略；新 corral 装上后自然生效。

## 先读
- 仓库根的 `CLAUDE.md`（规矩，尤其「独立编译目录」「桌面窗口测试」「验证」）。
- ranch 只读：`/Users/firegnu/Developer/personal_projs/ranch/docs/调研/R50-corral回答终端颜色查询.md` §4。不改 ranch。
- `docs/DESIGN.md`：P5-143a（终端回答颜色查询）、P5-162（不在前台时去色）、P5-90／P5-124 远程开 agent、P5-180（子进程 PATH）。
- 代码：`app/src/theme.rs` 的 `own_terminal()`；`app/src/window.rs` 应用新配置、更新 `chosen` 那段；`app/src/remote_start.rs` 的 `start`、`Request::command`；`app/src/corral.rs`；`app/src/settings_view.rs` 主题那一块和 Diagnostics。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-181-colors`，分支 `p5-181-colors`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-181-colors`（已从 `.target/main` 克隆）。命令在 `app/` 下跑。
- 并行：`paddock/dev-marks-1` 在 `p5-149-marks` 做 P5-149（终端层、注入脚本、`view.rs`、`viewer.rs`、`agent_shell.rs`）；你不要动这些文件。

## 要做的
1. **颜色**：取 `Theme::own_terminal()` 的前景、背景，写成小写 `#rrggbb`。不用画出来的那份（paddock 不在前台时会去色）。
2. **本机**：启动时调一次 `corral terminal-colors --fg … --bg …`；Settings 改了主题、终端颜色跟着变时再调一次。只在前景、背景和上次送出的不同时才调；几个窗口共用一份配置，不要每个窗口各调一次。放后台跑，有超时；失败（`usage`、找不到 corral、超时）不弹提示、不挡任何操作，Diagnostics 里如实显示最近一次结果（送了什么、成功或为什么没成）。
3. **跟着系统切深浅**：paddock 现在没有这个功能，这一条不做；以后加了在同一处调。
4. **远端**：`remote_start::start` 开远端 agent 时，在 `corral start` 之前对那台的 corral 调一次（用连接时找到的那个 corral）。可以拼进同一条远端命令（颜色那段输出丢掉、失败不管，再跑 start），不多一次 ssh 往返；照现有的引号写法；`corral start` 的回答照旧解析。远端旧版 corral 报错照常起。
5. **Settings 说明（第 4 条）**：在 Settings 选主题的地方加一行小字（英文），意思是已经在跑的 agent 不会跟着换主题，要重开才跟上，例如 `Agents already running keep the colours they started with; restart them to follow a new theme.` 沿用那一页现有说明文字的样子。
6. **文档**：`docs/DESIGN.md` 加一条「把终端颜色告诉 corral（P5-181）」，写用户原话和定下的做法。

## 怎么算做完
用户的验收原话是需求里的 1 到 4 条和“加第4条，现在就做”。逐条：
1. paddock 启动时、Settings 换主题时，用当前终端主题的前景、背景调了一次 `corral terminal-colors`（颜色没变不重复调）。
2. 远端开 agent 前，对那台的 corral 也调了一次。
3. corral 不认这个命令或调用失败时，开 agent、换主题都照常，不弹错。
4. Settings 选主题处有那行说明。
- 验证只做这些（改行为档）：针对性测试用假 `corral` 脚本记下收到的参数：启动调一次、换主题调一次、颜色没变不调、用的是没去色的颜色、失败（退出码 1 的 `usage`）不挡；远端命令里颜色那段在 start 之前、引号对、颜色那段失败时 start 的回答照旧解析。`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 真机效果（新 corral 装上后开 Claude Code、Codex 看是不是浅色）留给主控和用户。

## 不要做
- 不运行真的 `corral terminal-colors`（会改本机全局设置，影响用户和别的主控开的 agent）；测试只用假 `corral` 脚本和隔离的环境。不连远端机器。
- 不开任何 corral agent，不碰 `corral ls` 里已有的 agent（不 stop／send／keys）。
- 不改 ranch、Saddle 仓库；不用 Python。
- 不开 paddock 窗口、不截图；不用脚本模拟按键或鼠标。
- 不动 P5-149 在改的文件（见「在哪里干活」）。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。停自己起的进程用启动时记下的 PID。
- 碰到要拿主意的事，停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的办法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-181-colors` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- 做了什么：新增 `corral_colors`，取 `Theme::own_terminal()` 的前景／背景转小写十六进制；主窗口初始化和应用配置时共用 GPUI 全局去重状态，后台按顺序调用公开命令，每次限时 5 秒。失败只记 Diagnostics，那里显示送出的颜色、时间、发送中／成功／失败原因，并随结果更新。远端启动在同一条 SSH 命令内用连接时找到的 corral 先设颜色，丢弃该段输出、忽略失败，再执行原来的 start。Settings 主题处加英文说明，DESIGN §13 记录用户决定和做法。
- 验证了什么：先跑针对性测试确认 RED（本机启动未发送颜色；远端记录只有 start），实现后两项均 GREEN。假 corral 验证启动一次、重复窗口／相同颜色／只改字体不重复、换主题发送一次、去色不改变原色、小写格式、usage 失败后后续主题仍可处理；远端假脚本验证颜色先于 start、含引号和 shell 特殊字符的路径正确传入、颜色命令报 usage 仍返回 start 的结果，既有假 SSH 测试也确认每次启动仍只需一次 SSH 命令往返。`app/` 下指定独立编译目录，`cargo test --all-targets`（1049 passed）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次并全部通过；Cargo 仍提示既有依赖 `block 0.1.6` 的 future-incompatibility。命令均在前台等待结束。
- 拿主意的地方：沿用现有 `corral::Client` 和子进程超时／PATH 机制；本机调用串行，避免较旧的颜色设置晚完成而覆盖新主题。没有超出任务范围的决定，没有需要主控决定的事。
- 没做的事：没有运行真实 `corral terminal-colors`、连接远端、启动 agent 或 paddock 窗口；没有改 ranch、Saddle 或 P5-149 指定文件。没有加跟随系统深浅，没有合并或推送。新 corral 装好后的 Claude Code／Codex 真机效果留给主控和用户。
- 踩过的坑：没有。
- 返工（主控审查，10-24）：修正失败也记作已送出的错误，只有成功才更新去重颜色，去重判断放在前一条调用完成后；全局监听 `Startup.login_path` 变为 Ready，按当前主题补发。原测试只检查手动驱动的去重状态，漏了失败后重试，现在启用固定版本 GPUI 的 `test-support` 开发特性，直接以无窗口测试驱动实际 `sync`、后台任务和 Startup 通知（锁文件新增测试特性的依赖，未升级既有依赖）。两个新增测试先 RED 再 GREEN，覆盖 usage 后同色重试、成功后的排队去重、找不到命令后 PATH Ready 用最新主题重发；本次只跑 `cargo test --lib corral_colors::tests -- --nocapture`（3 passed）、`cargo test --lib launch::tests`（2 passed）、`cargo test --test login_path`（3 passed），未重跑全套及静态检查。DESIGN 同步更正，无须主控另作决定。

## 主控审查

- 第一轮看过 diff：颜色取 `own_terminal()`（没去色的那份）；几个窗口共用一个队列、先后送出，旧主题不会盖掉新主题；远端在 `corral start` 前拼一段 `terminal-colors … >/dev/null 2>&1 || :`，不多一次 ssh，引号照现有写法；Settings 主题下加了那行说明；Diagnostics 有一行 Terminal colors。重跑 `cargo test --all-targets` 1049 个通过，和对方说的一致。
- 返工 1 次（「怎么算做完」第 1 条在开机场景没达到）：原来失败也记成已送出。开机自动打开时（P5-180 的场景），PATH 还没取到、找不到 corral，这次失败后颜色不变就不再发；旧版 corral 换成新版后也不会补发。交回后改成只有成功才算送出、登录 shell 的 PATH 变成 Ready 时用当前主题补发，加了两条测试。复核：`corral_colors`（3 个）、`remote_start`（19 个）测试通过，clippy `-D warnings`、`fmt --check`、`git diff --check` 都过。
- 没做：真机效果要等 ranch R50b 部署新版 corral 后，和用户一起开 Claude Code、Codex 看是不是浅色。
结果：返工 1 次
