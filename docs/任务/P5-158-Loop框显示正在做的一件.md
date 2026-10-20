# 任务：看板 Loop 这块重新排：正在做的一件留在框顶上（Now），排着的在下面（Next），去掉单独的状态行

2026-10-20，paddock/main 交给 paddock/dev-loopui（Claude Code，sonnet / high，分工表第 8 条试用第 1 件，套 fence `dev` 墙）。
依据：用户 10-20 看了两轮 Loop 实测的截图：“loop的这个ui能否再设计一下，我觉得稍微有点乱”。主控画了样稿「Paddock 看板 Loop 样稿 2」（A／B／C 三种，各四种状态），用户：“我也同意A。建立任务吧。之后我送到loop中”。本件由看板 Loop 送来开工。
执行：派出
（理由：已有组件照选定样稿重排，不新做交互，落在分工表第 8 条，试用 sonnet；主控审，装上给用户试。）
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.95）、不要、影响面拿不准，主控按拿不准定改行为）
类型：样式／文案调整
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 现在是怎样（主控 10-20 对着代码和用户截图核对的）
- `app/src/kanban_view.rs` 的 `loop_group`：标题行（循环箭头、`Loop`、个数、要你处理时的 `Needs you` 胶囊、开关），下面一行状态字（`kanban_loop.rs` 的 `Stand::words`，如 `On · P5-155 sent to the controller`、`On · P5-156-P5 in progress · 1 next`），再下面一个框放 LOOP 里排着的任务。
- 乱在三处（用户截图上看得到）：
  1. 状态行从图标那一列起头，和标题文字 `Loop` 不对齐；用正文颜色和字号，看着像第二个标题；开头的 `On ·` 和右边的开关重复。
  2. 正在做的那件会跳走：刚送出时还在框里（记号是循环箭头），开工后框里就没了，跑到下面 In progress 里（带 `Loop` 标签），框里只剩“下一件”。
  3. 同一个框里两种记号：送出的那件是循环箭头，排着的是 ①。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-158-loop-now`，分支 `p5-158-loop-now`（已从 main 建好）。只动 `app/src/kanban_view.rs`、`app/src/kanban_loop.rs`，要的话 `docs/DESIGN.md` §13 P5-106 那段补一句界面怎么排。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-158-loop-now`（从 `.target/main` 克隆）。

## 要做的（照样稿 A）
1. **框分两段**：
   - **Now**：Loop 送出去、还没收尾的那一件（`Loop.sent`）固定在框顶上，不管它在哪一栏（Queued、In progress、To review、Merged）。记号是一个转着的小弧（强调色）；要你处理时换成琥珀色的 `!` 圆圈，框边也变琥珀色（沿用现在 `needs` 时的框边）。编号、标题照普通卡片（标题最多两行）。
   - 标题下面一行写它现在怎样，代替原来的状态行：
     - 送出了、还没开工：`Sent to <主控名字> · <多久>`。
     - 开工了：照普通卡片底下那一行显示它的 agent（头像、名字、状态、时长）；`执行：主控` 的显示主控，和 In progress 卡片一致。
     - 任务文件有 `待用户：`：`Needs you · <待用户的内容>`（琥珀色）。
     - 卡住：`Needs you · not wrapped up, controller idle 15m`（琥珀色），下一行 `Nudge`、`Skip` 两个按钮。
     - 提醒发不出去（`Failed`，nudge）：琥珀色一行写原因，按钮照现在的。
   - **Next**：Now 下面一条细分隔线，左边小号大写 `Next`，下面是 LOOP 里排着的，记号统一用 ①②③（现在的数字圆点），悬停按钮照旧（上移、下移、移出）。没有 Now 时不画 `Next` 分隔线，直接列。
2. **去掉单独的状态行**。没有 Now、但 Loop 在等的情况（等别的活收尾、没有主控、不止一个主控、排着的都还不能开工、主控忙一会儿再送、正在送、送出去失败要 Retry），放在框顶、Now 的位置，一行淡色小字（要你处理的用琥珀色），`Retry` 按钮照旧跟在后面；文字沿用现在 `Stand::words` 的说法，但去掉开头的 `On ·`，`Waiting:` 这类开头保留。
3. **关着时**：
   - 自己做完关掉的：框里只一行淡色字 `Finished · <N> done. Add queued tasks to run another loop.`
   - 手动关着、LOOP 里还有任务：只列 Next（不画分隔线），标题行图标变灰（现在就是），不另加状态行。
   - 关着、LOOP 里也没有任务：框里一行淡色字 `Add queued tasks to the loop.`
4. **标题行**：循环箭头、`Loop`、个数、开关。个数 = Now 那一件（有的话）加上排着的件数。标题行不再挂 `Needs you` 胶囊（框边和 Now 那行已经说明了）；看板别处、收件箱的 Needs you 照旧。
5. **列表和展开成五列两种排法都照这个结构**（`Place::List` 和 `Place::Tile`）。
6. 下面各栏里那张卡的 `Loop` 标签照旧，不动。

## 怎么算做完
- 用户原话：“loop的这个ui能否再设计一下，我觉得稍微有点乱”“我也同意A”。
- 主控建议的验收（请用户确认）：
  1. 针对测试：给定各种 Loop 状态（关着做完、关着有任务、关着没任务、刚送出、开工了、待用户、卡住、送失败、等别的活收尾、没有主控），框顶显示的是 Now 那一件还是哪一行字、Next 有没有分隔线、个数多少，都对。
  2. 三项检查全过（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。
  3. 用户装上后实际用 Loop 走一件，看各个状态下这一块是不是不乱了。
- 验证只做这些：第 1 条的针对测试，三项检查各跑一次（`CARGO_TARGET_DIR` 用上面写的目录）。第 3 条留给用户，你不起窗口、不截图。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 Loop 的规则：什么时候送、什么算做完、什么算卡住、开关、存哪里，都不动（`kanban_loop.rs` 里只许改状态说成什么字、给界面用的取值，不改 `look`、`settle` 的判断）。
- 不改各栏卡片、`Loop` 标签、收件箱、其他看板按钮的样子和行为。
- 不加新的主题颜色键，不加依赖。
- 不复制 Zed 的代码（GPL）。
- 不用 Python；命令里不用 `rm`、不用 `sh -c` 包长命令；不往 `/tmp` 写东西（墙里不让写）；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不起 paddock 窗口，不碰 `corral` 里现有的 agent，不对任何 agent `corral send`。
- 不合并到 main，不推送。只在 `p5-158-loop-now` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或墙的行为和预想不一样的；没有就写「没有」），在你的分支里提交。主控审过后装上给用户试，用户说可以再合并。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
