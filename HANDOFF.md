# 交接

## 现在在哪（2026-10-07 晚）

- main 和 `origin/main`（`github.com/firegnu/paddock`，public）同步，工作区干净。没有进行中的活，没有开着的 worktree，本地只有 main 一个分支。corral 里是三个主控：`paddock/main`（本主控）、`ranch/main`、`cairn/main`（后两个是用户开的，不动）。
- 328 项测试通过，clippy、`cargo fmt --check` 干净。`~/Applications/paddock.app` 是合并 P5-33b 之后的版本（含 P5-29b～f Kanban 加强、P5-30 底部提示、P5-31 记住窗口大小、P5-32 活动格子图、P5-33 agent 暂停、P5-33b 暂停状态处处一致），已用本机 Apple Development 证书签名。
- 全局 corral 已是 ranch `5c5540c`（支持 pause／resume）；三个主控的管理进程都已原地换新。
- 重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录见 AGENTS.md，主仓库用 `.target/main`）。打包默认自动选本机 Apple Development 身份，可用 `PADDOCK_SIGN_IDENTITY` 指定；退回 ad-hoc 时会提示授权会失效。
- 已合并：第一至第三阶段、迁移 M0–M3（corral、dispatch 在 `../ranch`；遥测、Drover、插件系统已砍）、P4-1～P4-3、P5-1～P5-33（P5-13 做了 13a 外壳、13b／13c Changes；P5-19 磨砂；P5-23r、P5-13r、P5-29r、P5-29r2 是调研；P5-28 做了 28a～28d；P5-29 做了 29a～29f；P5-33 做了 33、33b）。每件的范围、完成记录、主控审查在 `docs/任务/`。
- 上下文：`AGENTS.md`（规矩，含「开发方式」里的**看板约定**）、`docs/DESIGN.md`（已定决定，§13 是界面改版的全部决定和用户原话）、`docs/背景与决策记录.md`、`docs/设计稿/`、`docs/调研/`。上一次会话（10-06 晚到 10-07 下午：Browser P5-28、Kanban P5-29、P5-21～P5-27 等）的细节都在各自的任务文件里。

## 本次会话（10-07 下午到晚上）：agent 暂停

用户要“pause 是暂停而不是停止，不让其对外有连接”，选“冻结”；用法是活都做完后（下班、网络不好）一键冻住全部 agent，第二天恢复，不用一个个关掉重开。

- **原理**（向用户解释过，用户担心碰 Claude Code／Codex 底层）：系统信号 `SIGSTOP`／`SIGCONT`，和 Ctrl+Z、合上笔记本同一类；不改 agent 程序、配置和文件，对 claude、codex、pi、omp 都一样。
- **ranch R1（主控自己做，ranch 规矩如此）**：corral 加 `pause`／`resume`。冻 agent 的子孙和这些进程所在会话里的所有进程，冻完逐个确认停住；`status`／`ls` 多 `paused`；暂停中 send／keys 退回 10、接入窗口打的字丢掉、`wait`／`send --after` 不当作做完、stop 先恢复、升级保持暂停。Codex 交叉审查三轮：第一轮 7 条必须改都改了；R1、R6 交用户——用户**接受**“冻结那一刻自己另开会话又失去父进程的守护进程会漏”（macOS 没有系统级冻结一组进程的手段），**同意**部署次序（先切链接、再 `upgrade --all`、确认没有旧版 wait／提醒进程，之后才用暂停）。真 claude 实测两次通过。设计在 ranch `docs/DESIGN.md` §6，任务和审查在 ranch `docs/任务/R1-*.md`。
- **部署（用户在场）**：主控打包 `5c5540c`；切 `~/.local/bin/corral`、`corral upgrade --all`、`corral install-skills` 由用户在普通终端里跑（主控切链接被 auto 模式拦下，见“悬着”）；主控核对三个主控全部 complete、没有旧版 wait／`__after` 进程。
- **paddock P5-33（派 Claude Code xhigh）**：卡片 Paused 变淡、卡片和左下角菜单单个 Pause／Resume、侧栏头部铃铛左边一键暂停／继续（全部暂停后变 ▶，有干活或状态不明的先确认）、暂停的窗格不接受输入并浮 Resume。代价：侧栏最小宽度 224→254（字号 18 约 318）。
- **P5-33b（主控自己做）**：窗格标签点、命令面板、铃铛计数、活动格子图都认暂停（改读 `Panel::shown`）；Kanban 仍按 corral 的 state。
- **用户实测单个暂停通过**：测试用 Codex 和它的 15 个子孙进程（含另开会话的 node）暂停后全部 `T`，恢复后同一批进程回到 `S`、进程号不变，接着正常回答；测试 agent 已关。
- 顺带回答：活动格子图 10-04 及以前是空的，因为 cairn、paddock、ranch 三个仓库的第一个提交都在 10-05；Saddle 没进仓库列表（左侧栏没显示过它的 agent）。

## 下一步（按优先级）

1. **用户试一键暂停**（铃铛左边 ⏸）、确认框（agent 干活时点 Pause）和悬停动效。一键会冻住包括主控在内的全部 agent，要用户点 ▶ 恢复。有问题先修。
2. **用户实测 P5-28c、P5-28d**（用户 10-07：“我打算用到再测试”）：P5-28d 点一下 Browser 的 ×；P5-28c 照 `docs/任务/P5-28c-Browser网页策略与查找.md` 完成记录末尾 8 条清单（第 1 条本地地址最要紧）。
3. **等用户在真窗口里看**：Browser、Kanban（P5-29a～f：状态推得对不对、Needs you、DRAFT、Clear 的确认）、P5-30 底部提示、P5-31 记住窗口大小、P5-32 活动格子图（动画、悬停卡片、折叠）、Changes、P5-22、P5-24、P5-25、P5-26／27 动效。
4. 之后：Changes 第二步（行上评论发给 agent、暂存、撤销，另议）、拖动分隔线调整分屏大小、`paddock ctl`；Kanban 能动手的（新建草稿、拖来纠正，用户：排在后面）；Servo 作 Browser 备选（不做）。

## 悬着

- **auto 模式拦下主控改 `~/.local/bin`**（判为修改共享资源），连 `ls ~/.local/bin` 也被拦过一次。以后 ranch 部署（切链接、`upgrade --all`、装技能）照这次：主控打包、核对，用户在普通终端里跑这三步。
- **ranch／Saddle**：用户说 Saddle 基本不用了，Saddle 不跟暂停；`ranch/main` 问用户“Saddle 保底”规矩要不要放松，用户还没答（见 ranch `HANDOFF.md`）。
- **要不要把 `cargo fmt --check` 写进 AGENTS.md 的验证清单**：主控问过多次，用户还没答；任务文件里已经各自写上。
- 窗口窄、左侧栏展开时标签名被截得很短（“pa…”），原有问题；侧栏最小宽度因 P5-33 变大后应好一些。要不要处理待用户定。
- 截图能用，但 `PADDOCK_NO_ACTIVATE` 起的测试窗口不在前台，悬停、动画、键盘鼠标交互、全屏切换仍只能靠用户实际操作。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 观察：Claude Code 带的 `caffeinate` 冻住后“不睡眠”断言仍在，暂停 agent 不等于让 Mac 能睡。
- 建议改未排：Kanban 悬停按钮条盖住 Needs you 标记后半截；P5-21 的 `--model` 在命令末尾无值时会清掉已认出的模型、`--config=…` 连写不认；DESIGN §13 的 P5-20 条重复了一遍；New Agent 窗口 “Will run” 预览要重开才换字体；About 窗口在很大字号时可能放不下；配置里 `sidebar_width` 小于新最小宽度时不自动加宽；侧栏铃铛紧凑与否按估算字宽判断；左侧栏开关向外滑时短横稍压外框；地址栏没有撤销（`text_input.rs` 不支持）。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式（P5-23 只做了本机签名，仍不公证、不分发）。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
- Saddle 仓库里的 `t76-*` 分支、worktree 和 T76 状态由 Saddle 主控处理。
- 主控教训：
  - 派活时写明“命令里不用 `rm`、不用 `sh -c` 包长命令”；`corral start --unique` 会给名字加 `-1`，后续 wait／send／stop 用返回的名字；关 agent 前确认它是 idle 且 `attached` 为 0；release 构建很快结束时，核对产物时间晚于合并再安装。
  - `send --after` 的第一个参数是**收件人**：提醒自己写 `corral send "$CORRAL_NAME" … --after <对方>`。这次误挂过一条发给审查员自己的，靠按 PID 停掉它的 `__after` 进程止住；corral 没有撤销提醒的命令。
  - 新 worktree 第一次开 Codex 会卡在“是否信任这个目录”，要用户去点。
  - 测试 agent 截图用临时 HOME、假 corral、`--bounds` 和预写的布局文件（主控自己的简便做法：临时 `HOME`／`CFFIXED_USER_HOME`／`XDG_STATE_HOME`／`XDG_CONFIG_HOME` 加 `PADDOCK_NO_ACTIVATE=1 GPUI_TERM_WINDOW_ID=1` 先起一次 debug 版写出 `layout.json`，按 PID 停掉，改 `right_sidebar` 再起，截图后同样按 PID 停）；调研类只把文档摘到 main（`git cherry-pick`），原型分支不合并。
  - corral 的 socket 路径有长度上限，单独的 `CORRAL_HOME` 放在 scratchpad 会报 `path_too_long`，用 `/tmp` 下的短目录。
