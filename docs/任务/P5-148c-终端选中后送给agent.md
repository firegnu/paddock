# 任务：终端里选中文字后浮出小工具条（Copy、Send to agent），选一个 agent 送过去

2026-10-20，paddock/main 交给 paddock/dev-send（Claude Code，opus[1m] / high，套 fence `dev` 墙）。
依据：用户 10-20：“1，2，3，4，5，7一块做吧”，本件是其中的 7（1、2、3、8 是 P5-148a，4、5 是 P5-148b）。样稿上用户选：“7选第二种”，主控追问后确认三项都选第二种：工具条在松开鼠标的地方；送给谁都能挑，这个窗格的 agent 排第一；送的是代码块加来源一行。草稿 P5-54 是 Browser 和 Changes 里选中后送给 agent，和本件同类但不在本件。
依赖：P5-148b
执行：派出
（理由：新交互，终端层加工具条和事件，窗口里加挑 agent 的小面板并经 corral 送出，跨几个模块；交给 Claude Code 照样稿做，主控审。）
路由：重 / 交叉审查不要 / 影响面：改行为（路由：重（置信 0.31）、拿不准、拿不准；主控定：送的是用户自己选的文字、送给用户自己挑的 agent，不碰数据模型、并发、权限，按改行为、不交叉审查）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 现在是怎样（主控 10-20 核对的）
- 终端里拖选、双击、三击能选中，⌘C 复制（`view.rs`、`view/press.rs`）；选中后界面上没有别的入口。
- 往 agent 送话：Inbox 的回复框经 `corral send` 送（`window_inbox.rs`、`corral.rs` 的 `send`）。
- 小面板的样子有现成的一套（`popover.rs`）。

## 要做的（照样稿，用户选的）
1. **工具条**：在终端里用鼠标选中文字、松开后，在松开的地方旁边淡入一个小工具条：`Copy`、`Send to agent…`。选区没了（点别处、打字、输出把选区冲掉、按 Esc）工具条就走；工具条不挡住选中的字。程序要了鼠标时（选区走的是报给程序那条路）不出。
2. **Send to agent…**：点开一个小面板：
   - 列出可以送的 agent，这个窗格自己的 agent（如果是 agent 窗格）排第一并标 `this pane`，其余照侧栏顺序；shell 窗格里没有“自己的”，直接列其余的。上下键、回车能选。
   - 一个可选的备注框（`Add a note (optional)`）。
   - 预览要送出去的文字。
   - `Send` 送出，`Cancel` 或 Esc 关掉。
3. **送什么**：代码块加来源一行：
   ```
   From <窗格名，如 shell · ~/projects/paddock 或 dev-terminal · claude>:
   ```<选中的文字>```
   <备注，有才加，空一行>
   ```
   （代码块里的文字照选区原样，去掉行尾多余空格。）
4. **送出**：经 `corral send` 送给选中的 agent（远程机器上的 agent 照现有远程送话的路子）；成功后给一个短提示（`Sent to <agent>`），失败照 Inbox 送不出去时的说法提示，选区和备注不丢。
5. 只有一个 agent 可送时也照样列出来让人确认，不直接送。

## 怎么算做完
- 用户原话：“1，2，3，4，5，7一块做吧”“7选第二种”（三项都选第二种）。
- 主控建议的验收（请用户确认）：
  1. 有自动测试先红后绿：送出文字的组装（来源行、代码块、备注、行尾空格、选区里本来就有 ``` 时怎么办）；列表顺序（自己的 agent 排第一、shell 窗格没有自己的）；什么时候出工具条、什么时候走；送出失败不丢内容。测试里的送出用假 `corral`，不碰真实 agent。
  2. 三项检查全过（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。
  3. 用户实际用：在 shell 窗格和 agent 窗格里选一段，送给一个测试 agent，看到它收到的文字。
- 验证只做这些：上面第 1 条的针对测试先红后绿（送出用假 `corral`），三项检查各跑一次（`CARGO_TARGET_DIR` 用下面写的目录）。第 3 条留给用户，你不开真实 agent、不起窗口。觉得不够，在回复里说，不要自己加。

## 不要做
- **守住终端层的边界**（DESIGN §2）：终端层只出工具条和一个“这段要复制／要送出去”的事件（带选中的文字和窗格名），不认识 corral、侧栏、agent 列表；挑 agent、组装、送出都在窗口这边。
- 不改已有的选择、复制、⌘ 点链接、拖选自动滚的行为。
- 不做 Browser、Changes 里的送出（那是 P5-54）。
- 实测只用自己开的 `paddock/test-<名字>` agent（`--label role=test`），用完 `corral stop`；不对 `corral ls` 里用户的 agent `send`、`keys`、`stop`。
- 不加新的主题颜色键；不加锁文件里没有的依赖；不改 `gpui-pre-*`、alacritty_terminal、portable-pty 的版本。
- 不复制 Zed 的代码（GPL），只参考思路。
- 不起 paddock 窗口、不截图、不用脚本模拟按键或鼠标。
- 不靠删测试、放宽断言、缩短超时来通过。
- 界面文字用英文。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。不按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 你在 fence `dev` 墙里：被墙拦住、换个办法也绕不过去的，停下来在回复里说，不要想办法绕墙。
- 不装 paddock.app（`bundle --install` 由主控审过后来做）。
- 不合并到 main，不推送。只在 `p5-148c-send` 上提交，提交说明用中文、首行以 `P5-148c：` 开头。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-148c-send`，分支 `p5-148c-send`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-148c-send`（从 `.target/main` 克隆）。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么、验证了什么（哪些测试先红后绿）、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或墙的行为和预想不一样的；没有就写「没有」），在你的分支里提交。主控审过后装上给用户试，用户说可以再合并。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

（Claude Code，opus / high，fence `dev` 墙内，10-20）

### 做了什么
- 终端层（`view/toolbar.rs` 新、`view.rs`）：鼠标选中松开后，在松开处旁边淡入工具条 `Copy`、`Send to agent…`（120ms 淡入上浮，同 1A；系统减弱动态效果时只淡入）。往下拖放在选区最后一行下面，往上拖放在第一行上面，一边放不下换另一边，两边都放不下贴网格底；选区整个滚出视野时不画。按下鼠标、打字、Esc、选区被冲掉、窗格失去键盘时工具条走；程序要了鼠标、按下报给程序时不出（⇧ 拖选照样出）。`Copy` 同 ⌘C；`Send to agent…` 发 `SendSelection { text, pane, at }`（选中的文字、`subject()` 那个窗格名、工具条在窗口里的位置），终端层不碰 corral、侧栏、agent 列表。另给窗格加了 `tell()`：底部提示和 `say()` 一样，只是边框用强调色（`view/notice.rs` 加了 `fine` 标记）。
- 组装和列表（`send_to.rs` 新，纯逻辑）：`message()` 组装 `From <来源>:` + 代码块 + 备注；`source()` 写来源（shell 的目录家目录写 `~`，agent 加 kind）；`targets()` 排列表（窗格自己的 agent 第一并标 own，其余照 `Panel::ordered` 即侧栏顺序，去掉已结束的）；`Panel` 管选中哪个、发送中、被拒的原因。
- 窗口（`window_send.rs` 新、`window.rs`、`window_motion.rs`）：新浮层 `Popup::Send`，从工具条位置挂出（`popover::hang`），`Send to` 标题、agent 行（kind 图标加状态点、名字、kind、`this pane` 小标签）、备注框 `Add a note (optional)`、等宽字体预览、`Cancel`／`Send`，底部提示 `↑↓ choose ↩ send esc cancel`。↑↓、↩、Esc 走已有的 DIALOG 键位；点外面关。淡出时和别的浮层一样留着画完（`Leaving` 多存一份）。送出走 `corral::Client::send`（远程机器用该机器的 link，同 Inbox），后台跑；成功关面板、窗格底部 `Sent to <agent>`；被拒面板不关，文字、备注、选中的 agent 都在，下面用 `inbox::refusal` 的说法和 Inbox 一样的琥珀色写原因；面板已关时结果写在窗格底部（失败为 `Couldn't send to <agent>: <原因>`，红边）。
- `docs/DESIGN.md` §13 加了 P5-148c 一条。

### 验证了什么
- 先红后绿（红都是断言失败，不是编译错）：
  - `send_to::tests`（8 条）：来源行＋代码块＋备注、空白备注不加；行尾空白和末尾空行去掉、中间空行保留、`\r\n`；选区里有 ``` 时围栏加长；来源写法；列表顺序（自己的排第一、shell 窗格没有自己的、自己的已结束或不在列表时不标、只有一个也列出）；↑↓ 在列表里走、列表变了仍记着选的那个；发送中不重复发、被拒保留文字备注和选中项、再发清掉原因、成功结束；**经假 `corral`**：被拒（`not_idle`）、corral 没给 JSON，两种都保留文字和备注并给 Inbox 的说法；成功时假 corral 收到的是 `send <名字> <整段文字>`（文字是一个参数）。
  - `view::toolbar::tests`（4 条，其中 3 条先红；“普通点击和报给程序时不出”是反向断言，桩子下本来就过）：拖选、双击、三击、⇧ 拖选松开出工具条并淡入；普通点击、报给程序的按下不出；选区没了、失去键盘、按下／打字／Esc（`hide`）后走，走了不会自己回来；放置（下面、上面、换边、两边都放不下、左右夹住、部分滚出视野、全滚出视野不画）。这些测试用真的 `press::Mouse` 走松开流程，确认“程序要了鼠标”那条路不出工具条。
- 三项检查：`cargo fmt --check` 过；`cargo clippy --all-targets -- -D warnings` 过；`cargo test --all-targets` 945 个全过（见下面“踩过的坑”里那一个环境问题的处理）。
- 没起窗口、没截图、没开真实 agent（按任务要求，用户实测留给用户）。

### 拿主意的地方（请主控看）
1. **Esc**：工具条在时 Esc 只收起工具条并清掉选区，**不发给程序**。理由：agent 窗格里 Esc 会打断 Claude Code 等，用户想关工具条不该顺手打断 agent。工具条不在时 Esc 照旧发给程序。
2. **Copy 后**：复制到剪贴板、工具条走、选区留着（同 ⌘C）。`Send to agent…` 后同样选区留着，面板关掉后回到窗格还能 ⌘C。
3. **列出谁**：只列这个窗格所在机器的 agent（侧栏显示的就是这台机器的），去掉已结束（Exited）的；忙着的、暂停的也列出来，送不出去时按 Inbox 的说法说原因（和 Inbox 不同，Inbox 是不空闲就不让打字）。
4. **来源行**：agent 写 corral 里的全名加 kind（如 `paddock/dev-terminal · claude`），不是侧栏的短名；shell 写完整目录、家目录写 `~`（样稿例子 `shell · ~/projects/paddock`），不是标签上的缩写。
5. **代码块**：去掉每行行尾空白外，还去掉了末尾的空行（三击选整行时选区以换行结尾，不去会在代码块末尾多一空行）；开头的空行和中间的空行保留。备注去掉首尾空白，全是空白就当没有。
6. **面板关了而送出还在路上**：结果改在原窗格底部提示，不再弹面板。
7. **提示样式**：`Sent to <agent>` 复用窗格底部原有的“链接打不开”那条提示，加了强调色边框的版本（没加主题颜色键），5 秒。

### 没做的事
- 跨机器挑 agent（只列窗格所在机器的）。
- Browser、Changes 里的送出（P5-54）。
- 用户实测（验收第 3 条）。工具条的位置、淡入、面板的外观和按键都只经编译和逻辑测试，没在窗口里看过。

### 踩过的坑
- `view::links::tests::a_shell_s_relative_paths_are_from_where_it_has_gone_to` 在墙里单跑、全量都失败（“the shell did not get there”）。P5-148a 的完成记录里记过同一件事：PATH 里 ServBay 的 `clear` 在 `/usr/bin` 前面，不认 `xterm-256color`。照那里的办法 `PATH="/usr/bin:/bin:$PATH"` 跑全量全过。想在基线上核对时 `git stash push -u` 被墙挡（“Cannot save the current status”，写不了主仓库的 stash），没有绕，靠 P5-148a 的记录和 PATH 修正后通过来判断与本件无关。
- `.target/main` 里只有 10-17 的主程序，没有能直接拿来跑的基线测试程序。
- 在墙里 `git commit` 成功后 git 还报一句 `Unable to create .../paddock/.git/packed-refs.lock: Operation not permitted`（整理主仓库 refs 被墙挡）；提交本身已落到 `p5-148c-send`，没有处理。

## 主控审查（10-20）
- 范围：终端层只加工具条和 `SendSelection` 事件（`view/toolbar.rs`、`view.rs`，提示条 `notice.rs` 加成功样式），不认识 corral、侧栏；组装、列表、面板状态在 `send_to.rs`（纯逻辑），面板和送出在 `window_send.rs`，经 `corral::Client::send`（远程走 `machines.link`，同 Inbox）。没动已有的选择、复制、⌘ 点链接、拖选自动滚；没加主题键、依赖。agent 名字实际是 `paddock/dev-send-1`，套 fence `dev` 墙。
- 要做的五条逐条对过：工具条在松开处、不挡字、该走时走、程序要鼠标时不出；面板列表自己的排第一标 `this pane`、↑↓↩、备注、预览、Send／Cancel／Esc；送出格式（来源行、代码块、备注；围栏比选区里最长的反引号多一个）；成功提示、被拒留住文字和备注；只有一个也列出来。
- 重跑（墙外，`.target/p5-148c-send`）：`cargo test --all-targets` 945 过 0 失败，和对方报的一致；clippy `-D warnings` 无警告；fmt、`git diff --check` 过。
- 对方的取舍：工具条在时 Esc 只收起不发给程序、列出没结束的 agent（忙的暂停的也列，送不出按 Inbox 说法写原因）、来源行写全名加 kind、去掉末尾空行、面板关了结果改在窗格底部提示，都同意。
- 结果：一次过
