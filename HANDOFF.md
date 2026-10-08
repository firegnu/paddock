# 交接

## 现在在哪（2026-10-08 上午）

- main 和 `origin/main`（`github.com/firegnu/paddock`，public）同步，工作区干净。没有进行中的活，没有开着的 worktree，本地只有 main 一个分支。corral 里只有 `paddock/main`（本主控）（用户 10-07 夜让开的 `paddock/codex`、`paddock/pi`、`paddock/omp` 已按用户要求关掉）；`ranch/main`、`cairn/main` 已不在。
- 428 项测试通过，clippy、`cargo fmt --check` 干净。`~/Applications/paddock.app` 是合并 P5-42 之后的版本（10-08 09:22 装，含 P5-35～P5-42、paddock ctl；用户已重启），已用本机 Apple Development 证书签名。
- **paddock ctl 已部署**（用户 10-07 夜在终端里跑完，主控核对过）：`~/.local/bin/paddock` 链到 app 里的程序；`paddock install-skills` 已装 `~/.claude/skills/paddock`、`~/.agents/skills/paddock`（带归属标记，与仓库源文件一致）；旧的 `~/.claude/skills/saddle` 已删。`paddock ctl instances` 找得到正在跑的窗口，`inspect` 认出 `paddock/main` 是 pane 1。重新打包安装后链接不用重建；技能正文改了要重跑 `paddock install-skills`。已开着的 agent 要重开才看到新技能。
- 全局 corral 已是 ranch `5c5540c`（支持 pause／resume）。
- 重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录见 AGENTS.md，主仓库用 `.target/main`）。打包默认自动选本机 Apple Development 身份，可用 `PADDOCK_SIGN_IDENTITY` 指定；退回 ad-hoc 时会提示授权会失效。
- 已合并：第一至第三阶段、迁移 M0–M3（corral、dispatch 在 `../ranch`；遥测、Drover、插件系统已砍）、P4-1～P4-3、P5-1～P5-42（P5-13 做了 13a 外壳、13b／13c Changes；P5-19 磨砂；P5-23r、P5-13r、P5-29r、P5-29r2 是调研；P5-28 做了 28a～28d；P5-29 做了 29a～29f；P5-33 做了 33、33b；P5-34 做了 34、34b；P5-36 做了 36a～36c；P5-39 做了 39a～39d）。每件的范围、完成记录、主控审查在 `docs/任务/`。
- 上下文：`AGENTS.md`（规矩，含「开发方式」里的**看板约定**）、`docs/DESIGN.md`（已定决定，§13 是界面改版的全部决定和用户原话）、`docs/背景与决策记录.md`、`docs/设计稿/`、`docs/调研/`。上一次会话（10-06 晚到 10-07 下午：Browser P5-28、Kanban P5-29、P5-21～P5-27 等）的细节都在各自的任务文件里。

## 10-08 上午：P5-42 输入框中文输入法崩溃

- 用户 09:07 崩溃（日志在对话里，不入库）：系统拼音输入法调 `setMarkedText` 时 `TextInput` 切字符串越界，`extern "C"` 里不能展开，整个 app abort。原因是从 GPUI `examples/input.rs` 带来的选区算法错（按整个输入框换算、结尾加 `range.end`），输入法清空拼写中文字（删光拼音、Shift 切换）后再打字就崩。终端窗格不受影响。
- 主控自己修（用户“写完就做”），先写失败测试再修，合并、推送、重新打包安装。用户重启后试过，没问题（10-08）。

## 10-07 晚上：P5-35～P5-40、paddock ctl

细节都在各自的任务文件里，这里只记结果。

- **P5-35** 拖动分屏之间的缝调大小（悬停亮线、双击恢复对半，比例存进布局）。
- **P5-36a** 几处小毛病：Kanban 按钮条不盖 Needs you、About 大字号放得下、过窄的 `sidebar_width` 自动加宽、铃铛按实际字宽判断紧凑、左侧栏开关动画不压外框。**P5-36b** `--model`／`--effort` 末尾缺值不清空、Codex `--config=` 连写能认；单行输入框 ⌘Z／⇧⌘Z。**P5-36c** Kanban 按钮条不挡增删行数和编号、时间；布局里的坏比例按对半用。
- **P5-37** New Agent 重做（样稿 B：预设一排、先写任务的大输入框、底边一排选项、Show command），种类图标用本机真实原图。
- **P5-38** 标签名不被截短：挤时留能分辨的那段、从中间省略，再挤收进“+N”菜单，当前标签始终可见。
- **P5-39a～d paddock ctl**（照搬 saddle ctl，另加 `browse`）：a 传输层和命令行、`install-skills`（Codex 交叉审查两轮）；b 界面端 inspect／open／close／browse、调用者定位、shell 注入身份、busy（交叉审查两轮）；c 技能正文；d 急修——用户退出后 paddock 打不开（WebKit 先建了 0755 的 `$TMPDIR/paddock`，ctl 要 0700 而拒绝，又因 ctl 失败退出整个程序），改用专用 `paddock-ctl` 目录，ctl 起不来时照常启动、左下角提示。
- **P5-40** 四套深色主题（Catppuccin Mocha、Tokyo Night、Rosé Pine、Kanagawa），设置里主题选择改成带色块的列表。
- **P5-41**（主控自己做）左侧栏卡片头像：自带底色的原图（Claude、Codex、omp）铺满 34pt 头像，不自带底色的（pi）保留底块、放大到 24pt；按“不透明占比＋接近正方形”自动判断。右下角状态不动；Kanban、标签栏不动（用户选）。**P5-41b** 窄条格子同样铺满，选中、等待改由格子外 3pt 一圈底板表示，格子间距 2→3.5pt。本机 Codex 图标换成 ChatGPT.app 的图标（用户要求；`~/.config/paddock/icons/codex.png`，不在仓库）。
- DESIGN 顺带记了两笔：浅色主题暂不做；组件库（gpui-component、Ely）暂不引入（用户 10-07）。

## 10-07 下午到晚上：agent 暂停、格子图改版

用户要“pause 是暂停而不是停止，不让其对外有连接”，选“冻结”；用法是活都做完后（下班、网络不好）一键冻住全部 agent，第二天恢复，不用一个个关掉重开。

- **原理**（向用户解释过，用户担心碰 Claude Code／Codex 底层）：系统信号 `SIGSTOP`／`SIGCONT`，和 Ctrl+Z、合上笔记本同一类；不改 agent 程序、配置和文件，对 claude、codex、pi、omp 都一样。
- **ranch R1（主控自己做，ranch 规矩如此）**：corral 加 `pause`／`resume`。冻 agent 的子孙和这些进程所在会话里的所有进程，冻完逐个确认停住；`status`／`ls` 多 `paused`；暂停中 send／keys 退回 10、接入窗口打的字丢掉、`wait`／`send --after` 不当作做完、stop 先恢复、升级保持暂停。Codex 交叉审查三轮：第一轮 7 条必须改都改了；R1、R6 交用户——用户**接受**“冻结那一刻自己另开会话又失去父进程的守护进程会漏”（macOS 没有系统级冻结一组进程的手段），**同意**部署次序（先切链接、再 `upgrade --all`、确认没有旧版 wait／提醒进程，之后才用暂停）。真 claude 实测两次通过。设计在 ranch `docs/DESIGN.md` §6，任务和审查在 ranch `docs/任务/R1-*.md`。
- **部署（用户在场）**：主控打包 `5c5540c`；切 `~/.local/bin/corral`、`corral upgrade --all`、`corral install-skills` 由用户在普通终端里跑（主控切链接被 auto 模式拦下，见“悬着”）；主控核对三个主控全部 complete、没有旧版 wait／`__after` 进程。
- **paddock P5-33（派 Claude Code xhigh）**：卡片 Paused 变淡、卡片和左下角菜单单个 Pause／Resume、侧栏头部铃铛左边一键暂停／继续（全部暂停后变 ▶，有干活或状态不明的先确认）、暂停的窗格不接受输入并浮 Resume。代价：侧栏最小宽度 224→254（字号 18 约 318）。
- **P5-33b（主控自己做）**：窗格标签点、命令面板、铃铛计数、活动格子图都认暂停（改读 `Panel::shown`）；Kanban 仍按 corral 的 state。
- **用户实测单个暂停通过**：测试用 Codex 和它的 15 个子孙进程（含另开会话的 node）暂停后全部 `T`，恢复后同一批进程回到 `S`、进程号不变，接着正常回答；测试 agent 已关。
- 顺带回答：活动格子图 10-04 及以前是空的，因为 cairn、paddock、ranch 三个仓库的第一个提交都在 10-05；Saddle 没进仓库列表（左侧栏没显示过它的 agent）。
- **格子图改版 P5-34（派 Claude Code high）**：用户嫌“酷炫不足、平淡”，给了 GitHub 贡献图截图。平淡的原因：强调色加透明度叠在深底上发闷、空格子几乎看不见、只有今天发光。主控出三张样稿（画布「活动格子图改版」，源文件 `docs/设计稿/P5-34-格子图改版/`），用户选 C：实色阶梯（强调色混进底色 → 强调色 → 往正文色混成白热）、最忙两档光晕、每 7 秒对角扫光、今天光圈呼吸更明显；只从主题取色。**P5-34b**（主控自己做，用户同意）：扫光只要窗口在前台就扫，不再要求有 agent 在干活；今天那格的呼吸仍要。
- **格子图名单**：用户问怎么让更早的日期有数据——名单只增不减，agent 关了仓库也照算；手动往 `~/.local/state/paddock/activity-repos.json` 加路径也行（paddock 每分钟重读并合并）。主控按用户要求加了 saddle、global-mesh。

## 下一步（按优先级）

0. **cairn 集成（10-08 和用户定的方案，四步）**：① ranch R2：corral-dispatch、corral 技能开出去的 agent 带 `CAIRN_DISABLE=1`（派活的各在 worktree、存了没用；问问题的开在仓库目录，会把停点存到主控那条线上）——已合并、部署（10-08 16:27，`d55defb`，技能核对一致）。② 已装（10-08 16:28 用户在终端跑 `cairn install --agent claude --yes`、owlet `cairn adopt`，只装 claude；主控用 `cairn status --json` 核对：4 个 hook、固定路径、save 规则都在，owlet adopted；settings 备份 `~/.claude/settings.json.bak-20261008T082855…`）。owlet/main 要重开才读到新技能、才被 cairn 挂上；用几天看续跑那一小轮顺不顺、接上的内容靠不靠谱。③ cairn 侧（用户转给 cairn 主控）：`status`/`show`/`list` 的 JSON 定成公开约定，`show` 分节、`list --json`，加只读开关；可与试点并行。④ paddock 一件活：右侧栏只读面板 **Recap**（用户确认：接续和保存都是自动的，面板只给人看；名字避开“Resume”免得像要点按钮），未装→说明＋可复制命令，装了未启用→Enable for this repo（`adopt`），已启用→停点、下一步、待用户决定＋Off（`unadopt`）；只在打开面板或切 agent 时调用、不轮询，用 cairn 固定路径；New Agent 加 “cairn off”（默认不勾）。不做：全局安装按钮、接进 Needs you、格子图数据源。等③完成、试点内容可靠后再做。
1. **用户在真窗口里试 10-07 晚上这批**：拖缝调大小和双击；⌘Z／⇧⌘Z 和输入法；New Agent 新对话框（点选项、⌘↩、预设增删）；标签“+N”菜单和悬停全名；四套新主题（Rosé Pine 光标偏暗，看前台实心光标是否够显眼）；ctl 的 busy（Browser／Kanban 确认时）、`--focus` 的键盘去向、`--attach` 启动。**P5-36c 待用户定**（不挡合并）：加宽五列悬停时卡片变高一行、下面的卡片下移，还是平时就给每张卡片留出按钮那一行。
2. **用户看格子图新样子**（扫光、白热光晕、呼吸；数据多了 saddle、global-mesh 后应更满）。
3. **用户试一键暂停**（铃铛左边 ⏸）、确认框（agent 干活时点 Pause）和悬停动效。一键会冻住包括主控在内的全部 agent，要用户点 ▶ 恢复。有问题先修。
4. **用户实测 P5-28c、P5-28d**（用户 10-07：“我打算用到再测试”）：P5-28d 点一下 Browser 的 ×；P5-28c 照 `docs/任务/P5-28c-Browser网页策略与查找.md` 完成记录末尾 8 条清单（第 1 条本地地址最要紧）。
5. **等用户在真窗口里看**：Browser、Kanban（P5-29a～f：状态推得对不对、Needs you、DRAFT、Clear 的确认）、P5-30 底部提示、P5-31 记住窗口大小、P5-32 活动格子图（动画、悬停卡片、折叠）、Changes、P5-22、P5-24、P5-25、P5-26／27 动效。
6. 之后：Changes 第二步（行上评论发给 agent、暂存、撤销，另议）；Kanban 能动手的（新建草稿、拖来纠正，用户：排在后面）；Servo 作 Browser 备选（不做）。

## 悬着

- **auto 模式拦下主控改 `~/.local/bin`**（判为修改共享资源），连 `ls ~/.local/bin` 也被拦过一次。以后 ranch 部署（切链接、`upgrade --all`、装技能）照这次：主控打包、核对，用户在普通终端里跑这三步。
- **ranch／Saddle**：用户说 Saddle 基本不用了，Saddle 不跟暂停；`ranch/main` 问用户“Saddle 保底”规矩要不要放松，用户还没答（见 ranch `HANDOFF.md`）。
- **要不要把 `cargo fmt --check` 写进 AGENTS.md 的验证清单**：主控问过多次，用户还没答；任务文件里已经各自写上。
- 部署 paddock ctl 照“用户在终端里跑”的做法：链接、`install-skills`、删 saddle 技能都由用户做（10-07 已做完）。用户机器上旧的 `$TMPDIR/paddock`（WebKit 缓存、残留旧套接字）没动，不影响使用。
- 截图能用，但 `PADDOCK_NO_ACTIVATE` 起的测试窗口不在前台，悬停、动画、键盘鼠标交互、全屏切换仍只能靠用户实际操作。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问未保存的设置和运行中的 shell（GPUI 没有提供拦截）。
- 观察：Claude Code 带的 `caffeinate` 冻住后“不睡眠”断言仍在，暂停 agent 不等于让 Mac 能睡。
- 建议改未排：DESIGN §13 的 P5-20 条重复了一遍；New Agent 窗口 “Will run” 预览要重开才换字体（P5-37 重做后没再核对是否还在）。其余之前记下的（按钮条遮挡、`--model`／`--config=`、About 大字号、`sidebar_width`、铃铛判断、开关动画、输入框撤销、标签名截短）已由 P5-36a～c、P5-38 做掉。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式（P5-23 只做了本机签名，仍不公证、不分发）。
- Xcode 缺 Metal 工具链组件，目前靠 `runtime_shaders`；是否安装待用户决定。
- Saddle 仓库里的 `t76-*` 分支、worktree 和 T76 状态由 Saddle 主控处理。
- 主控教训：
  - 派活时写明“命令里不用 `rm`、不用 `sh -c` 包长命令”；`corral start --unique` 会给名字加 `-1`，后续 wait／send／stop 用返回的名字；关 agent 前确认它是 idle 且 `attached` 为 0；release 构建很快结束时，核对产物时间晚于合并再安装。
  - `send --after` 的第一个参数是**收件人**：提醒自己写 `corral send "$CORRAL_NAME" … --after <对方>`。这次误挂过一条发给审查员自己的，靠按 PID 停掉它的 `__after` 进程止住；corral 没有撤销提醒的命令。
  - 新 worktree 第一次开 Codex 会卡在“是否信任这个目录”，要用户去点。
  - 测试 agent 截图用临时 HOME、假 corral、`--bounds` 和预写的布局文件（主控自己的简便做法：临时 `HOME`／`CFFIXED_USER_HOME`／`XDG_STATE_HOME`／`XDG_CONFIG_HOME` 加 `PADDOCK_NO_ACTIVATE=1 GPUI_TERM_WINDOW_ID=1` 先起一次 debug 版写出 `layout.json`，按 PID 停掉，改 `right_sidebar` 再起，截图后同样按 PID 停）；调研类只把文档摘到 main（`git cherry-pick`），原型分支不合并。
  - corral 的 socket 路径有长度上限，单独的 `CORRAL_HOME` 放在 scratchpad 会报 `path_too_long`，用 `/tmp` 下的短目录。
