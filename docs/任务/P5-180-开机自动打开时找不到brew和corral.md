# 任务：开机自动打开的 paddock 找不到 brew 和 corral

2026-10-24，paddock/main 交给 paddock/dev-path（Codex，碰要害：gpt-6-astra / xhigh）。
路由：重 / 交叉审查不要 / 影响面：碰要害（路由：重（置信 0.98）、要、碰要害；推翻：交叉审查改为主控审，用户 10-19 定派出的活由主控直接审、不另开审查者）
类型：Bug 修复
依据：用户 10-24 无意重启机器后，paddock 终端里敲 `brew` 说没有这个命令，paddock 也说 corral 没有这个命令，退出 paddock 重开才好。主控调查过（见下），用户问影响面后说：“可以，写成草稿吧。”“按你说的排，先派 P5-180”。本轮只修取 PATH，不碰 P5-149（命令分段）。
执行：派出
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- “我刚才无意重启机器之后，在paddock的终端中输入brew，提示没有这个命令。paddock也提示corral没有这个命令。直到我关掉padodck的background，然后重新启动你之后就好了。”
- 主控列了影响面和改法后：“可以，写成草稿吧。”

## 先读
- 仓库根的 `CLAUDE.md`（规矩，尤其「独立编译目录」「桌面窗口测试」「验证」）。
- `app/src/main.rs` 开头到建 `NewShell` 那段、`app/src/launch.rs`、`app/src/command.rs`、`app/src/pty.rs` 的 `spawn_owned`、`app/src/viewer.rs` 起 shell 那段、`app/src/agent_shell.rs` 的 `setup`／`run`、`app/src/diagnostics.rs` 的 `Startup`。
- `docs/DESIGN.md` 里 P2-4（从桌面启动时向登录 shell 取 PATH）、P3-10（Diagnostics）、P5-61（Agent shell）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-180-path`，分支 `p5-180-path`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-180-path`（已从 `.target/main` 克隆）。命令在 `app/` 下跑。

## 主控调查到的（10-24，只读）
- 14:59 机器重启；15:00:09 macOS 登录时“重新打开窗口”拉起 paddock（系统日志 loginwindow `persistentAppPreLaunch`，同一批还有 Chrome、Slack、微信等）。
- 从桌面或登录时启动，paddock 拿到的是 launchd 的精简 PATH（`/usr/bin:/bin:/usr/sbin:/sbin`）。`main.rs` 在还只有一个线程时调 `launch::login_path` 跑一次 `$SHELL -l -c` 取 PATH，**只等 3 秒，超时就沿用精简 PATH，之后不再重试，界面不提示**（只在 Settings › Diagnostics 的内存结果里有一项）。
- 用户的登录 shell 要跑 `.zprofile`（brew shellenv、nvm、amazon-q 等），热机时 0.42 秒；开机头一分钟一批应用同时启动时超过 3 秒是推断，没有直接记录。
- 终端的 shell 是 `$SHELL -i`（非登录 shell，不读 `.zprofile`），PATH 继承 paddock 进程的；corral 也按 PATH 找。所以 PATH 没拿到时，终端缺 Homebrew 和 `~/.local/bin`，paddock 找不到 corral，直到退出重开。
- 没对上的一处：用户 `~/.zshenv` 现在有“没有 Homebrew 就补上”的一段，非登录 shell 也会读，按它终端里该有 brew。该文件修改时间是 10-24 15:09（出问题之后、重开之前），主控和派出的 agent 都没改过。要是这段是 15:09 才加的，就都解释得通；要是早就有，终端缺 brew 另有原因。待用户确认。

## 要做的（主控的建议范围，用户同意）
1. 取登录 shell 的 PATH 超时后不放弃：后台接着等或再取，拿到后给之后新开的终端和 paddock 自己调的命令（corral 等）用。已经开着的终端不用改。从终端启动的 paddock（不走取 PATH）行为不变。
2. 一直拿不到时，界面上说清楚（新开的终端和找命令用的是精简 PATH、怎么办），不再只是悄悄沿用。沿用现有提示的位置和样式，界面文字用英文。Diagnostics 里的登录 shell 那一项跟着如实显示（还在取、拿到了、没拿到）。
3. 有了别的线程以后不能改进程自己的环境变量（Rust 里不安全）：把 PATH 存在一处，起子进程（`command.rs` 等几处、`pty.rs` 开终端、找 corral 等程序的地方）时显式传进去。这一处会被多个线程读写，要保证读到的是完整的旧值或新值。
- 不做：终端改成登录 shell（每个窗格多跑一遍 `.zprofile`、PATH 可能重复，影响所有终端）；不改用户的 `.zshenv`、`.zprofile`；不改 ranch、corral。

## 怎么算做完
- 用户的原话就是验收：开机后自动打开的 paddock，终端里能用 `brew`，paddock 能找到 corral，不用退出重开。
- 验证只做这些（碰要害档）：
  - 针对性测试，先写、先看它因为这个 bug 失败：用假的慢登录 shell（超过原来的 3 秒）确认 PATH 取到后，新开的 shell 和找 corral 用的是登录 shell 的 PATH。
  - 边角用例：登录 shell 一直不结束、退出码非 0 或不打标记、打出空 PATH；拿到之前开的 shell 照旧、之后开的用新的；多个线程同时起子进程时读 PATH 不出错；拿不到时有提示。
  - `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。
  - 觉得不够，在回复里说，不要自己加。真机重启验证留给用户。

## 不要做
- 测试只用假登录 shell 和假 `corral` 脚本，不用本机真实的登录 shell、不读用户的 shell 配置文件（`~/.zshenv` 里有用户的密钥）。
- 不改用户的 shell 配置、不改 ranch、Saddle 仓库；不用 Python。
- 不重启机器、不注销，不改系统设置里的登录项。
- 不开 paddock 窗口、不截图（这件活界面改动小，留给用户看）；不用脚本模拟按键或鼠标。
- 不开任何 corral agent，不碰 `corral ls` 里已有的 agent（不 stop／send／keys）。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。停自己起的进程用启动时记下的 PID。
- 碰到要拿主意的事，停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等，不要自己换别的办法。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-180-path` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 还没问清的（不挡开工）
- `.zshenv` 那段是不是 15:09 才加的（见上）。要是早就有，终端缺 brew 另有原因，另查。
- 用户说的“关掉 paddock 的 background”指什么：退出 paddock，还是系统设置登录项里的“允许在后台”。

## 完成记录

- 做了什么：桌面启动仍最多前台等 3 秒，同一个登录 shell 在后台继续取 PATH，总限时 60 秒。成功后用 `RwLock` 整体替换共用值，`command::start`、PTY、Diagnostics 程序查找和 pony 查找均读取快照；普通终端和 Agent shell 随后新开的用新值，已有终端不变。移除了启动路径里的 `set_var("PATH", …)`，从终端启动、headless 命令和独立 Agent shell 替身保留调用方环境，显式的子进程 PATH 覆盖／移除仍优先。侧栏沿用列表错误提示的样式显示英文等待／失败说明，恢复时消失并刷新本机 corral 列表；打开着的 Diagnostics 同步更新 PATH 的等待、成功、失败状态。设计补在 P2-4 下。
- 验证了什么：先加 4 秒慢登录 shell 回归，运行 `cargo test --test login_path -- --nocapture`，原实现于 3.01 秒失败（实际 `None`，预期登录 shell 的 PATH），确认 RED 来自本次缺陷。实现后针对性 3 项测试通过：慢登录恢复、假 brew／corral、普通及 Agent shell、恢复前开的终端继续用旧 PATH、4 个线程跨恢复时同时启动命令且只能读到完整旧值／新值；一直不结束、非零退出但打印标记、无标记、空 PATH 均保留原 PATH 并提供说明；快速成功和不走桌面初始化的路径也通过。慢登录用文件信号确保恢复前终端已启动，测试在隔离 HOME／ZDOTDIR／PATH 的子进程里执行。
- 指定检查：`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次，全部通过；全量共 1047 项测试。编译目录均为 `.target/p5-180-path`，全量测试另设临时 ZDOTDIR 避免现有 zsh 测试读用户配置。只有原有上游 `block v0.1.6` 的 future-incompatibility 提示。所有命令前台运行并等到结束。
- 拿主意的地方：采用继续等待同一次登录调用，不重复执行 profile；总等待上限取 60 秒，仍失败时明确提示检查登录 shell 配置后重开或从正常终端启动。取 PATH 复用现有有界命令执行器（输出上限 1 MiB），让退出和读输出都受超时约束，并拒绝非零退出。无需要主控决定的事项。
- 没做的事：没有真机重启／注销、没有打开 paddock 窗口或截图，界面与开机体验留给用户实测；未读写用户 shell 配置，未启动真实 corral agent，未改 ranch／corral／Saddle、登录项或 P5-149。未安装、合并或推送；只在本分支提交。现装版本等长期事实未变，没有改 HANDOFF。上面两项用户用法的疑问保持原记录。
- 踩过的坑：没有。

## 主控审查

- 看过 diff：取到的 PATH 存在 `launch.rs` 的 `RwLock` 里整份替换；`command::start`（所有子进程都经过它，主控 grep 核对过，没有绕过的）和 `pty.rs` 开终端时显式传入，显式设了 PATH 的照旧优先；启动路径不再 `set_var`。前台仍等 3 秒，后台同一次调用总限时 60 秒。侧栏提示、Diagnostics 跟着结果更新。只动了任务范围里的文件，「不要做」一件没碰。
- 取舍：等同一次登录调用、不重跑 profile，同意；60 秒上限，同意；非零退出算失败（脚本最后一条是 `printf`，正常时退出码是 0），同意。
- 重跑：`cargo test --all-targets` 1047 个通过（主控没另设 ZDOTDIR，也全过），clippy `-D warnings`、`fmt --check`、`git diff --check` 都过。
- 建议改（不挡合并）：还在等的提示用的是 `agents_red`，和失败一样醒目；换成灰字可能更合适。要不要改由用户定。
- 没做：真机重启验证，留给用户下次开机时看。
结果：一次过

## 用户决定

- 10-24 用户：“P5-180收尾。” 没等下次开机实测就收尾；开机后要是还找不到 brew 或 corral，另开一件活。草稿末尾两个没问清的（`.zshenv` 那段什么时候加的、“关掉 paddock 的 background”指什么）没再追问。
