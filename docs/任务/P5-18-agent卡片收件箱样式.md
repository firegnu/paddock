# 任务：侧栏 agent 卡片改成收件箱样式，信息一项不少

2026-10-06，paddock/main 交给 paddock/dev-inbox（Claude Code，重：opus[1m] / xhigh）。
路由：重 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（重 0.72，置信 0.59），活大且要新加后台读回复，定为重；交叉审查拿不准（并发 0.53），读回复只是在现有轮询旁加一次公开命令调用，不碰数据模型和核心规则，定为不要；影响面拿不准（看得见 0.02），定为改行为）
类型：功能变更
依据：本轮改侧栏 agent 卡片（收起和展开）的样子，并新加“最后一次回复”和 Reply 按钮；不改窄条方块、Attention 列表、分组规则和排序。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话（10-06）

“关于这块我还是想精调一下，总感觉agent的信息感觉不精细。”看过第一版：“我的意思是不是信息的展示，而是这个排版和布局有点死板，不灵光的感觉，不是给人眼前一亮的那种感觉”。看过第二版：“按推荐，但是信息不能丢。这些信息我都需要”。

## 先读
- `docs/DESIGN.md` §13 最后一条「P5-18」。
- `docs/设计稿/P5-18-agent卡片/README.md`，再看 `Inbox.dc.html`（HTML 和末尾 `renderVals()`）；`Now.dc.html` 是现状对照。样稿有几处信息没画全，以本文件「信息清单」为准。
- `app/src/card.rs`（`Card`、`card()`、`second`、`Place`、`details`、`look`、`brand`、`click`）、`app/src/sidebar.rs`（卡片怎么画、`click`、`SidebarEvent`）、`app/src/kind_icon.rs`、`app/src/corral.rs`、`app/src/agents.rs`（轮询）。
- `corral guide` 里 `reply` 的说明（输出格式、没有回复时的返回）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-18-inbox`，分支 `p5-18-inbox`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-18-inbox`。
- 主要改 `card.rs`、`sidebar.rs`；读回复改 `corral.rs`、`agents.rs`；Reply 聚焦要动 `window.rs` 里处理 `SidebarEvent` 的地方。别的文件确有必要才动，在完成记录里说明。

## 信息清单（一项不能少）

现在卡片和展开明细上的每一项，在新样子里的位置：

| 现在 | 收起时 | 展开后 |
|---|---|---|
| 状态（圆点颜色、呼吸） | 头像右下角状态角标，颜色同现在 | 小格 STATUS：状态词 · 工具（同现在 Status 一行） |
| 工具（Read、Edit…） | 干活中、卡住时预览行开头，状态色 | 同上 STATUS |
| 时间（这一轮 / 空闲 / 等待多久） | 第一行右边，规则同现在 `card()` 的 `origin` | 小格 THIS TURN 或 IDLE FOR / WAITING FOR |
| 名字、未读（如现在有加粗等） | 第一行，同现在 | — |
| 在此窗口打开（现在的 ▭） | 名字后一个强调色小点，悬停说明 “Open in this window” | 小格 WINDOWS 里写 “· open here” |
| 种类（图标 + 词） | 头像里的种类图标（没有图标的种类用首字母），悬停说明种类名 | — |
| 强度（high…） | 第一行名字后，淡色小字 | 小格 MODEL：模型 · 强度 |
| 目录、分支、+增 −删、?未跟踪、↑领先 | 第三行：现在的位置行原样（`Place::spans`） | 小药丸：⎇ 分支、+增 −删、?未跟踪、↑n to push 或 clean；全路径一行 |
| 等待的问题 / 出错说明（现在的 `Second::Note`） | 预览行，颜色同现在 | 同左，展开到六行 |
| 模型 | — | 小格 MODEL |
| 接入窗口数（Attached） | — | 小格 WINDOWS |
| 上次输入（谁、多久前） | — | 小格 LAST INPUT |
| Instance | — | 原样显示（等宽小字），旁边 Copy 按钮复制它 |

新增：预览行（见下）、小格 UP（开了多久，`started`）、最后一次回复。

## 要做的

1. **头像块**：卡片左边 34pt 圆角方块（随界面字号缩放），底色是种类色的低不透明度，中间种类图标（`kind_icon`），没有图标的用名字首字母（同窄条 `initial`）。状态角标挂右下角，外圈一道卡片底色：干活中一段转圈（GPUI 的旋转动画）、等你琥珀色点轻闪、卡住橙色带 “!”、空闲小绿点、出错和退出红点、启动中和未知淡色点。颜色都用现在 `look()` 的。
2. **第一行**：名字（大一号，同现在的选中加粗规则）、在此打开的小点、强度（淡色）；右边时间，干活中、等你、出错时带状态色，空闲淡色。
3. **预览行**（最多两行，超出省略）：
   - 干活中、卡住：状态色的工具名 + 终端标题（去掉开头的转圈符号和空白；标题只是组名时不用，同现在的过滤）。
   - 等你、出错：现在 `Second::Note` 的文字。
   - 空闲：最后一次回复的开头。
   - 以上都没有时不显示预览行，位置行上移成第二行。
4. **位置行**：现在的位置行原样（目录 · ⎇ 分支、+增 −删、?未跟踪、↑领先，颜色同现在），小一号，作为第三行。
5. **最后一次回复**：agent 进入空闲或等你时，在后台调用一次 `corral reply <名字>`（经 `corral.rs`，和现有命令同样的方式），按“名字 + instance + 进入这个状态的时间”缓存；同一段空闲里不重复调；读失败或没有回复就当没有，不报错、不重试。不能阻塞界面和现有的 `corral ls` 轮询。
6. **等你的卡片**：底色换成琥珀色低不透明度，加一圈很淡的琥珀细边；预览下面一个 Reply 按钮：显示这个 agent（同点击卡片的 `Attach`）并把键盘焦点放进它的窗格。样稿里的 Open 按钮不要（和点卡片重复）。
7. **空闲的卡片**：没选中时名字和预览淡一档。
8. **分组头**：右边一条细状态条，每个 agent 一小段，按 等你、干活中、卡住、其他、空闲 排，颜色同状态色。
9. **展开**（触发方式同现在 `Click::Details`）：按「信息清单」：小药丸一行、两列小格（小号大写标签在上、值在下）、全路径一行、Instance 一行带 Copy、预览展开到六行、右下 Stop…（走现有的停止 agent 流程和确认）。样稿里的 Open 按钮不要。
10. 点击卡片的行为、排序、分组、窄条方块、Attention 列表都不变。

## 怎么算做完
- 用户原话：信息一项不能少——上面「信息清单」每一行在新样子里都看得到；样子以 `Inbox.dc.html` 为准，颜色从主题取，不新加主题颜色键。
- 1–10 条做到。
- 验证只做这些：
  - `card.rs` 里给“预览行怎么选”（干活有标题、干活没标题、等你、空闲有回复、空闲没回复）和“信息清单每项都进了展开明细”各写测试，先确认在现在的代码上失败或不存在，再实现到通过；
  - 读回复：用假 `corral` 脚本测一次“同一段空闲只调一次 reply”；
  - 在 `app/` 下跑一次 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`。
  - 可以按 AGENTS.md「桌面窗口测试」起一次窗口截自己的图看一眼，截不到就算了，截图不入库。
  觉得不够，在回复里说，不要自己加。

## 不要做
- 不改窄条方块、Attention 列表、窗格卡片、标题栏、弹出框；不改分组和排序规则。
- 不新增主题颜色键，不加依赖，不改 `Cargo.lock`。
- 只通过公开命令和 corral 打交道（`corral ls/status/reply`），不读 corral 的内部状态目录。
- `corral ls` 里的 agent 都是用户的：不对它们 stop/send/keys，不 attach 上去打字；测试一律用假 `corral` 脚本和合成数据，不调用真实的 `corral reply`。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`（会弹权限框，没人点就卡住）。
- 不要用 `osascript`、System Events 等任何方式模拟按键或鼠标。起窗口时加 `PADDOCK_NO_ACTIVATE=1`，只截自己开的窗口，用临时 HOME 和假 corral。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- GPUI 做不到某一条（例如转圈动画、两行省略），停下来报告，等决定，不要自己换别的样子。
- 不合并到 main，不推送。只在 `p5-18-inbox` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

- **做了什么**：
  - `card.rs`：卡片模型改成收件箱样式。`Preview`：干活中、卡住是工具加终端标题（标题去掉开头的转圈符号和空白，只剩组名时不用）；启动中、等你、出错、退出是原来 `Second::Note` 的文字和颜色；空闲是最后一次回复的开头（换行和空白合成一个空格，最多 800 字）；都没有时为 `None`。`place` 一直有（原位置行）。展开明细换成 `Details`：药丸（⎇ 分支、+增 -删（有二进制文件时接 “, n binary”）、?未跟踪、↑n to push、clean），小格（Status、Model、This turn / Idle for / Waiting for、Up、Windows、Last input），全路径，显示用的 instance。`Line::Group` 多带组内各 agent 的状态，按等你、干活中、卡住、其他、空闲排。`yield_to_name` 只让强度让位（工具不在第一行了）。去掉了 `Second`、`Face`、`Detail`、`branch_details`、`ago`。
  - `agents.rs`：`Spell`（名字 + instance + `state_started`）和回复表：`replies_to_read` 给出还没问过的空闲或等你时段并记为已问，时段结束就连回复一起忘掉；`absorb_reply` 只收仍在进行的时段；`reply` 取当前时段读到的回复。
  - `corral.rs`：`Client::reply`（跑 `corral reply <名字>`，取 `text`；出错或没有回复给 `None`）和后台线程 `Replies<K>`：一次读一个，结果走通道，界面线程只 `try_iter`；退出时取消正在跑的命令并收线程。
  - `sidebar.rs`：新卡片。34pt 头像（种类图标，没有图标的种类用名字首字母，种类色 13% 底，悬停显示种类名），右下角状态角标（干活中转圈的弧、等你琥珀点轻闪、卡住橙色带 “!”、其余小圆点，颜色都取 `look()`；外圈是卡片底色，悬停时跟着变）。第一行：名字 14pt（选中加粗规则不变）、未读点（不变）、在此打开的强调色小点（悬停 “Open in this window”）、强度淡色小字、右边时间（颜色规则不变）。预览由 GPUI 断行，两行省略、展开六行。位置行小一号放第三行，没有预览时上移。等你的卡片底色叠 7% 琥珀、加 22% 琥珀细边、带 Reply 按钮。空闲没选中时名字和预览淡一档。分组头右边状态细条。展开：药丸一行、两列小格、全路径（等宽）、instance 加 Copy、右边 Stop…。轮询拿到 `corral ls` 结果后问回复，读到的回复下一次轮询时取走。窄条提示改从新字段取同样的内容，样子不变。`Tip` 的文字改成 `SharedString`，头像的提示要显示种类名。
  - `window.rs`：新事件 `SidebarEvent::Stop(name)` 走原来的 `stop_agent`。`stop_agent` 加了一个可选名字参数，菜单和快捷键照旧停当前窗格的 agent。Reply 发的是原来的 `Attach`：`show_agent` 本来就以 `focus_active` 结尾，会把键盘焦点放进窗格，所以没有另加事件；卡片已选中时点卡片只开合明细，点 Reply 照样接入并聚焦。
- **验证了什么**：
  - 先写三个测试，在原代码上编译不过（`Preview`、`git_chips`、`Replies`、`replies_to_read`、`ask_replies` 等都不存在），实现后通过：
    - `the_preview_says_what_it_does_asks_or_last_said`：干活有标题、干活没标题（只有转圈符号加组名）、干活什么都没有、等你、空闲有回复、空闲没回复；
    - `everything_the_card_showed_is_still_on_it_or_in_its_details`：信息清单每一项；
    - `a_reply_is_read_once_for_each_spell_of_idling`：假 corral 给每次 reply 编号，同一段空闲里列了五次只调一次，换一段再调一次，日志正好两行。
  - 旧测试跟着新字段改：第二行改成预览和位置行，工具从预览里取，让位只剩强度。删了已被新测试覆盖的 `details_name_the_internals_in_words`，以及随 `ago` 一起去掉的 `input_ages_read_as_words`。
  - `app/` 下 `cargo test --all-targets` 全过（共 207 项，库 176 项）；`cargo clippy --all-targets -- -D warnings` 无警告（只有上游 `block v0.1.6` 的提示）；`cargo fmt --check` 通过；`Cargo.lock` 没变。
  - 用临时 HOME、临时 `XDG_STATE_HOME`、假 corral（六个合成 agent）和 `PADDOCK_NO_ACTIVATE=1` 起了自己的窗口。`screencapture -x -o -l <编号>` 报 “could not create image from window”（同 P5-17），没截到图。进程按 PID 停掉，没有残留。
- **拿主意的地方**：
  - 启动中、退出也照旧用原来的 Note（“Starting…”、“Exited”）做预览，信息不丢；未知状态没有预览。等你的问题也用去掉转圈符号后的标题，和干活中用同一套清理。
  - Reply 放在位置行下面（卡片最底下），这样位置行仍是第三行，也仍在预览下面。
  - 药丸：增删写 “+25 -3”，减号和位置行一样；未跟踪写 “?3”；领先写 “↑2 to push”；没有改动、也没有未跟踪文件时加 “clean”，可以和领先同时出现（同样稿）。
  - 小格：Model 没有模型和强度时写 “—”（同样稿）；Up、时间格没有起点时不显示。Windows 写 “none” 或数字，在此窗口打开时加 “ · open here”。Last input 写成 “You · 2m ago” 的短格式，两列里放得下。小格里的时长用两个单位（“4m 12s”、“1h 12m”），第一行的时间仍是原来的短格式。
  - 等你时也按任务第 5 条读一次回复，但等你的卡片显示的是问题，读到的回复不显示。
  - 取色：名字 `agents_text`（空闲没选中时 `agents_branch`）；预览 `agents_branch`（空闲没选中时 `agents_dim`）；小格标签 `agents_dimmer`、值 `agents_text`；路径 `agents_dimmer`；药丸底是 `agents_text` 5.5%；Stop… 是 `agents_red` 85%。琥珀底叠在卡片原底色上，选中的等你卡片仍看得出选中。
  - 尺寸：头像、图标、角标、字号随界面字号缩放；卡片内边距和圆角照现有习惯不缩放。分组状态条每个 agent 12pt，最多 96pt。
- **没做的事**：
  - 没截到图。卡片、头像、角标动画、Reply、展开的实际样子要用户在窗口里看；Reply 聚焦、Copy、Stop… 也没实际点过。
  - `footer_icon` 的 `Icon::Here` 现在没人用了（模块是 pub 的，不报警告），按任务范围没动。
  - 没改 DESIGN（§13 P5-18 已写了这个样子）；没打包、没安装、没合并、没推送。
