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
