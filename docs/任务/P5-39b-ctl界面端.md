# 任务：paddock ctl 的界面端——inspect、open、close、browse 在窗口里生效

2026-10-07，paddock/main 交给 paddock/dev-ctlui（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查要 / 影响面：碰要害（主控定：和 P5-39a 同属一件，关闭含运行 shell 的窗格和调用者身份是要害）
类型：功能变更
依据：本轮把 P5-39a 留下的界面接口接上，让命令真正在窗口里生效；不改传输层和命令行的约定（确有必要先停下来报告）。
依赖：P5-39a
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」。
- `docs/DESIGN.md` §3 第 5 步、§13「paddock ctl（P5-39）」。
- `docs/调研/P5-39-paddock-ctl方案.md`（第 2 节命令、第 4 节安全边界）。
- `docs/任务/P5-39a-ctl传输层与命令行.md` 末尾「完成记录」（给你的接口：`Server::process_pending(handler)`、处理者签名、`Records::update`／`get`／`values`、`Caller`、`Place`、`CloseTarget`）和 `docs/任务/P5-39a-审查.md`（交叉审查对接口的约定，如“返回 busy 前不能执行修改”）。
- Saddle（只读，`../saddle`，提交 `f7d1bbaf`）：`src/app_control.rs`（界面端的做法：调用者定位、`inspect` 的输出、`open` 的三种内容和相对位置、`close` 的确认凭据、`busy`、异步状态更新）、`skills/saddle/SKILL.md`（对外约定）。只参照，对着 paddock 重写。
- paddock：`app/src/control.rs`、`main.rs`（现在的 `unsupported` 处理者）、`window.rs`（窗口、标签、分屏、New Agent 的“开好再放到指定位置”、关闭窗格的确认、拖分隔线和对话框等“用户正在操作”的状态）、`layout.rs`、`browser.rs`／`browser_view.rs`（打开网址、本地地址补全）、`viewer.rs`／`pty.rs`（shell 启动和环境）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-39b-ctlui`，分支 `p5-39b-ctlui`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-39b-ctlui`。

## 要做的
用户 10-05：“这个也要迁移。”10-07：“继续吧。弄完再说。都做到这份上了”。
1. **调用者定位**：用 P5-39a 转发的身份认出“调用者自己的窗格”：corral 开的 agent 按 `CORRAL_NAME` 加 `CORRAL_INSTANCE`（实例不符就当认不出），paddock 开的 shell 按 `PADDOCK_INSTANCE` 加 `PADDOCK_PANE`。窗格换了内容以后，旧身份不能再指向它（照 Saddle 的 revision 思路，需要就给 shell 注入的身份带上版本）。认不出时，依赖“自己位置”的操作报错，不猜。
2. **给 shell 注入身份**：paddock 开的每个 shell 带上 `PADDOCK_INSTANCE`、`PADDOCK_PANE`（以及需要的版本）；不改 agent 的环境（它们走 corral）。
3. **inspect**：返回实例、当前标签和窗格、所有标签和窗格（种类、显示的 agent 或 shell 目录、分屏布局）、`caller.pane`（认得出时）。
4. **open**：`--place tab|left|right|up|down`，默认相对调用者自己的窗格，`--relative-to active` 或窗格 ID 可改；三种内容互斥：`--shell [--cwd]`（默认取来源窗格已知的目录）、`--agent NAME`（已在别处显示就挪过去，不重复接入）、`--name NAME [--cwd] [--role] -- PROGRAM ARG…`（经公开的 `corral start` 新开，复用 New Agent 的“开好再放”）；`--prompt` 只在显式给出时带上；默认不抢焦点，`--focus` 才切过去。异步的步骤先回 `accepted`／`starting`，完成或失败后用 `Records::update` 更新（`complete` 只表示显示好了）。
5. **close**：`--pane`／`--tab` 只断开 paddock 的显示，不停 agent。目标里有在跑的 shell 时，先回 `confirmation_required`、受影响的清单和一次性凭据，什么都不关；带 `--confirm-shells --confirmation 凭据`（和新的请求 ID）再来才关；目标变了凭据作废。
6. **browse**：在右侧栏 Browser 打开 http／https 网址（本地地址照 P5-28c 补全），右侧栏没开就打开它并切到 Browser；`--focus` 才把键盘交给网页。
7. **busy**：用户正在拖分隔线、开着对话框或确认框、New Agent 窗口正在创建时，修改类命令回 `busy`，且回之前不做任何改动（P5-39a 交叉审查约定）。
8. 不加任何发按键、读屏、停 agent、读网页内容的能力。

## 怎么算做完
- 上面八条达到。
- 验证（碰要害一档）：
  - 测试（不起真窗口能测的部分抽成纯函数测）：调用者定位（agent 身份、shell 身份、实例不符、窗格换了内容后旧身份失效、认不出）、open 的位置计算（五种 place、relative-to self／active／ID）、`--agent` 已显示时挪动而不重复接入、close 的确认流程（有 shell 先要确认、凭据一次性、目标变了作废、确认后才关）、busy 时不改动、browse 只收 http／https。边角：同时两个修改请求、窗格在请求处理中被关掉。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 实测一次（不模拟按键鼠标）：release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`／`XDG_RUNTIME_DIR`（短路径）、假 corral，起一个 paddock，用编出来的 `paddock ctl` 依次跑 inspect、open（shell 到右边、新开一个假 agent 到新标签）、browse 一个 `http://127.0.0.1:端口`（自己起的临时服务）、close 一个带 shell 的窗格（先拿到确认要求，再带凭据关），每步截一张自己的窗口看过；按 PID 停掉。截图放 scratchpad，不入库。
- 写完后主控会请另一个 Codex 做只读交叉审查。

## 不要做
- 不改 P5-39a 的传输层和命令行约定（需要改先停下来报告）；不改技能文字（P5-39c）。
- 不碰用户真实的配置、布局文件、运行目录、技能目录、`~/.local/bin`，不连用户正在运行的 paddock 实例。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys/pause，不 attach 上去打字；测试只用假 corral。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不加依赖；不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-39b-ctlui` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么

- **调用者定位**（`control_ui::caller_pane`）：有 `CORRAL_NAME`／`CORRAL_INSTANCE` 时只按它认：两项都要有，sidebar 最近一次 `corral ls` 里同名同实例且无错误，窗口里显示它的窗格正连着、连的是这个实例；否则报错。没有 corral 身份才看 shell 身份：`PADDOCK_INSTANCE` 等于本实例、`PADDOCK_PANE` 的窗格还开着、显示 shell、shell 活着、且这个 shell 启动时注入的正是这对身份。认不出时 `inspect` 的 `caller` 给 `{"pane":null,"error":…}`，依赖自己位置的 `open` 回 `caller_unresolved`，不猜。
- **shell 身份注入**：`Launch::Shell` 和 `TerminalView::start_shell` 多带环境；窗口为每个 shell（启动、恢复布局、New shell、`+`/分屏的 Shell、ctl 开的）注入 `PADDOCK_INSTANCE`、`PADDOCK_PANE`，走原有的 `Session::spawn_shell`（它已清掉继承的 corral／Saddle 身份）。agent 环境不动。
- **“换了内容旧身份失效”**：不加版本环境变量（P5-39a 的 `Caller` 只转发四项，加字段要改传输层和命令行约定），改为：唯一一处“在已有窗格里起 shell”（New shell 落在空的活动窗格）先给窗格换一个新 ID（`Workspace::renew`），所以窗格 ID 和它里面的 shell 一一对应；其余内容变化（shell 退出后放进 agent 等）因“必须显示活着的 shell”而失效。另给每个窗格一个 `revision`（`Workspace::revision`，内容每次设定就涨），用于 inspect、请求进度和关闭确认。标签有了运行期 ID（`Tab.id`，不存进布局）。
- **inspect**：实例、当前标签和窗格、`zoomed`、每个标签的 ID、活动窗格、分屏树（`{"split":"row|column","ratio",…}`）、每个窗格的 ID、revision、种类（shell／agent／command／empty）、agent 名和 corral 实例、已知目录、状态、shell 程序和退出码，以及 `caller`。
- **open**：`Workspace::open_at`／`move_pane` 在任意窗格旁放新窗格或新标签（紧跟该窗格所在标签之后），不改活动标签和活动窗格；新终端创建时会抢键盘，不带 `--focus` 时还原到原来的焦点，带 `--focus` 才切过去。`--shell` 目录默认取来源窗格已知目录（shell 的启动目录、agent 的目录），没有则取 paddock 启动目录，结果写 `cwd_source`；`--agent` 必须在 corral 列表里，已在窗口里显示就把那个窗格（同一 ID、同一接入）挪过去，原接入已断才重新 attach；`--name … -- PROGRAM` 用 New Agent 同一个 `new_agent::start` 在后台跑公开 `corral start`，回 `starting`，成功后（用户不忙时）才在原锚点旁开窗格并 attach，记录依次更新为 `attaching`→`complete`；锚点在这期间关掉就记 `target_invalid` 并写明 agent 仍在跑；`--prompt` 只在显式给出时带上，role 收 regular/controller/implementer/reviewer。
- **异步结果**：窗口记下在途请求，`main.rs` 每 10 ms 调 `process_pending` 后再调 `control_tick`，把变化用 `Records::update` 写回：显示好了是 `complete`（不表示模型就绪），接入或 shell 失败是 `failed`，窗格被关或换了内容是 `target_invalid`。
- **close**：只断开显示（agent 窗格 `close` 只放开 attach）。目标里有活着的 shell 时回 `confirmation_required`、受影响清单（窗格、revision、种类、是否在跑 shell、目录、程序、agent）和一次性凭据，什么都不关；带 `--confirm-shells --confirmation` 再来才关；凭据用过、或目标清单与当时不同（窗格被换、shell 退出、revision 变了）都作废，作废后不会复活。关闭时只有键盘原本在被关窗格里才把焦点给新的活动窗格。
- **browse**：打开右侧栏、切到 Browser、按原工具栏同一条路径打开网址；`--focus` 才把键盘交给网页；只收 http／https（传输层已校验，这里再判一次）。
- **busy**：拖左右分隔线、拖分屏缝、拖窗口标题栏、开着系统确认框（`asking`）、开着任何面板或菜单（新标签／分屏面板、命令面板、Attention、New task、Actions、`+N`）、New Agent 窗口正在 Create、正在退出时，修改类命令在做任何事之前回 `busy`（不消耗确认凭据、不记录）；ctl 新开的 agent 已启动但用户正忙时，开窗格这一步也等用户忙完再做。
- 没有加发按键、读屏、停 agent、读网页的能力。P5-39a 的 `unsupported` 处理者只剩传输层自己的测试在用，改为 `#[cfg(test)]`。

### 验证了什么

- 新增纯函数测试（不起窗口）：`control_ui` 14 项——agent 身份、shell 身份、corral 身份优先、实例不符／半个身份／列表里有但没显示／别的 paddock／窗格不开／窗格不是 shell／接入已断／shell 已退出都认不出、窗格换内容和换 ID 后旧身份失效、inspect 输出、五种 place × relative-to self（agent 和 shell）／active／ID、目录来源、`--agent` 已显示时挪动不重复接入（接入已断才重连）、新 agent 的参数和 prompt、关闭确认（先要确认、两个开关都要、一次性、别的目标的凭据无效、目标变了作废、无 shell 直接关）、busy 时什么都不改且不花凭据、browse 只收 http/https、同一时刻两个修改（两次 open 各得新窗格、同一凭据两次只成一次）、请求进行中窗格被关／被换内容→`target_invalid`、接入成功→`complete`、失败→`failed`；`layout` 3 项（任意窗格旁开、移动保留 ID 并关掉空标签、revision 和 renew）；`tests/pty.rs` 1 项（注入的身份真的进了 shell 环境）。这些是新功能，测试和实现一起写，没有先单独跑出 RED。
- 在 `app/`、`.target/p5-39b-ctlui` 下前台跑 `cargo test --all-targets`（418 项全过，库测试 357 项）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，`git diff --check` 通过。
- 实测（release、`PADDOCK_NO_ACTIVATE=1`、`GPUI_TERM_WINDOW_ID=1`，HOME／XDG_CONFIG_HOME／XDG_STATE_HOME 在 scratchpad，XDG_RUNTIME_DIR 用短的私有 `/tmp/p39b.pVHA`，假 corral 脚本，`nc` 起的 `127.0.0.1:18765` 本地页面；没有模拟按键鼠标）：`instances`／`inspect`（无身份时 caller 报错；带 shell 身份认出 pane 1；带假 agent 的 corral 身份认出 pane 3，换个实例号认不出）；`open --place right --shell`（相对自己）→ `starting` 后查询为 `complete`，活动窗格仍是 1；`open --relative-to active --place tab --name live/helper --cwd /tmp -- claude --model sonnet` → 假 corral 收到的正是 `start live/helper --cwd /private/tmp --label role=regular -- claude --model sonnet`（没有 `--prompt`），随后新标签里接上、查询为 `complete`，活动标签没变；`browse 127.0.0.1:18765` → 右侧栏打开到 Browser、页面加载（截图里看得到）；`close --pane 2` → `confirmation_required` 与清单，未关；带凭据再关 → `complete`，pane 2 的 zsh 进程已结束；同一凭据再用 → `confirmation_invalid`。最后按记下的 PID 停掉 paddock 和页面服务（及其 nc），残留子进程一并确认已结束。
- 截图在 scratchpad `live/shots/`（不入库）。窗口被别的窗口挡住时 GPUI 暂停绘制，几张是旧画面：`2-…`、`2b` 还没画出右边的 shell，`3-…` 有分屏但还没画出 helper 标签，`6-closed`／`6b` 仍画着已关掉的 pane 2；状态以 `inspect`、`request` 和保存的布局文件为准（都与预期一致）。`4-browse.png` 是完整的一帧（分屏、helper 标签、Browser 页面都在）。关闭后的实际画面和“不抢焦点”的手感留给用户实际看。

### 拿主意的地方

- shell 身份不带版本号，改用“在已有窗格里起 shell 时换窗格 ID”，以免改 P5-39a 的协议；效果是 New shell 落进空窗格后该窗格的 ID 会变（inspect 里可见）。主控若更想要 Saddle 式的 `PADDOCK_REVISION`，需要先改传输层和命令行约定。
- 新 agent 照任务说的“开好再放”：先 `corral start`，再开窗格，所以 `starting` 阶段 `pane` 为 null（Saddle 是先占一个窗格）。
- `open --agent` 的 agent 已显示但接入已断时，挪过去后重新接入（不算重复接入）；已显示且接入中／已接上则只挪。
- busy 把所有弹出面板和菜单都算“开着对话框”，也把标题栏拖窗口和正在退出算进去；Settings 窗口自己的对话框不算（它不影响主窗口布局）。
- `browse` 在右侧栏已开但停在别的页签时也切到 Browser（为了让网页看得见）。
- 实测时 `XDG_RUNTIME_DIR` 放在 `/tmp/p39b.pVHA`（scratchpad 路径太长，套接字路径会超 104 字节），里面留着一个 SIGTERM 后没删的旧套接字，传输层会忽略它；按要求没有用 `rm`。实测里 Browser 用的是 WebKit 默认数据存储（非 .app 的 paddock 进程），本地页面的缓存可能留在系统的 WebKit 目录。

### 没做的事

- 没改传输层和命令行约定、技能文字（P5-39c）；没加依赖；没改 DESIGN（实现没有偏离已定设计，上面第一条请主控定是否写进 §13）。
- busy 的各种用户状态（拖分隔线、对话框、New Agent 创建中）只做了纯函数测试，没有实测（实测需要模拟鼠标键盘，按规矩不做）；`--focus` 时键盘的去向、关闭后焦点的去向同样留给用户实际体验。
- 没合并、没推送，只在 `p5-39b-ctlui` 分支提交。

### 交叉审查后的修改

依据主控认可的审查 R1、R2、R3（必须改）和 R4（建议改）。每条先写测试、对着当时的行为跑出断言失败（为了能编译，先加了保持旧行为的空壳：`Track.instance`、`Workspace::touch`、`busy_reason`／`Activity`、`landing`、`viewer::public_metadata`），再实现转绿。

- **R1 重新接入是新的一代**：窗口的 `attach` 每次都先 `Workspace::touch` 提升窗格 revision（ctl 的 `--agent` 重连、侧栏点击重连、新开窗格的首次接入都走它）；纯移动（`move_pane`，沿用原接入）不变。关闭确认清单的每个 agent 窗格带 `corral_instance`；在途请求记下当次接入的实例，同一窗格接上同名别的实例时记 `target_invalid`，revision 变了也一样。RED：同名新实例、同目录、同窗格、同 revision 时，旧凭据仍得到 `Close`；旧请求被新实例记为 `complete`。
- **R2 busy 读主窗口的实际确认状态**：busy 改为纯函数 `control_ui::busy_reason(&Activity)`，窗口按当下状态填：主窗口挂着系统 sheet（`NSWindow.attachedSheet`，覆盖 Browser 的 JS alert／confirm／prompt 和文件选择器，也包括 paddock 自己的确认框）、Kanban 有卡片在问 Clear（新增 `KanbanView::confirming`），以及原有的拖动、`asking`、面板／菜单、New Agent 创建中、退出中。新请求和晚到 start 的落位都用这一个判断。RED：`sheet`／`kanban_confirming` 为真时确认关闭仍得到 `Close`；测试同时核对 busy 时凭据没花掉、布局没变。sheet 的读取本身要真窗口，没有实测。
- **R3 `paddock --attach` 的实例**：`Launch::Agent` 带上 metadata；启动时先经公开 `corral status NAME`（5 秒上限，`viewer::public_metadata`）取实例交给 Viewer，接入前照常核验（未退出、实例一致、没在别处接着）。`caller_pane` 的实例检查没有放宽；corral 答不出时照旧接入，只是这个窗格不能当 ctl 的调用者。RED：`tests/viewer.rs` 按 `Launch::Agent` 的同一流程用假 corral 接入到 running 后，`caller_pane` 返回 caller_unresolved（实例为 None）。注意两点行为变化：带 `--attach` 启动多一次 `corral status`（最多等 5 秒才开窗）；拿到实例后，若该 agent 已在别处接着，启动接入会像侧栏点击一样被拒。
- **R4 晚到的 start**：落位前用 `control_ui::landing` 按名字＋实例重查：没显示就新开窗格；已显示同一实例就把那个窗格挪到锚点旁并跟踪；显示的是别的实例（或 corral 没给实例）就记 `failed`／`agent_conflict`，说明创建的 agent 仍在跑。RED：已显示同一实例时仍判为新开。

验证：在 `app/`、`.target/p5-39b-ctlui` 下前台跑 `cargo test --all-targets`（423 项全过，新增 5 项）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，`git diff --check` 通过。这一轮没有重新做桌面实测；sheet／Kanban 确认时的 busy、`--attach` 启动的实际效果留给用户体验。没改传输层、命令行约定和技能文字。

## 主控审查

- 八条达到；对方用 release 程序和假 corral 把 inspect、open（shell 到右边、新开 agent 到新标签）、browse、close（先确认、带凭据再关、凭据不可重用）实跑过。主控同意“shell 在已有空窗格启动时换新窗格 ID”（不改 P5-39a 协议）。
- 交叉审查（Codex xhigh）3 条必须改（R1 同名不同实例重新接入不换代次，旧凭据和在途请求可误认新实例；R2 busy 没读 Browser 原生 sheet 和 Kanban Clear 确认；R3 `--attach` 启动的 agent 缺实例、认不出自己）、1 条建议改（R4 晚到 start 可能重复显示），主控都认可、都交回改。修改后主控核对：每次接入都换代次、关闭快照带实例、在途请求绑定当次实例；busy 改为读主窗口 `attachedSheet` 和 Kanban 确认状态，新请求和异步落位共用；`--attach` 经公开 `corral status` 取实例（最多 5 秒）；晚到 start 落位前重核名字和实例。每条先有失败的测试。主控在分支上重跑 `cargo test --all-targets`（423 项全过）、clippy、fmt。
- 和 P5-39a 一样，先合并，审查员在后台复核；复核若有必须改，由 dev-ctlui 补修再合入。
- 留给用户试：Browser／Kanban 确认时的 busy、真实 `--attach` 启动、焦点与键盘去向。
