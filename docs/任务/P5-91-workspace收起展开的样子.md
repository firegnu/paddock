# 任务：workspace 收起时画成一摞、图标换成 agent 的叠卡，展开时托盘做成凹槽

2026-10-14，paddock/main 交给 paddock/dev-workspace-look（Claude Code，常规档：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：常规、不要、看得见）
类型：样式／文案调整
依据：用户 10-14 看过装好的 P5-89 后：“1: 展开和收缩的图标一样，我觉得应该改一下。2: 展开的情况下，workspace的展开状态（ui层面不明显，不好看出来这是一个展开的workspace）。3: 收缩情况下现在ui太普通了。能不能做一个类似之前agent区域的tab按钮的那个stack的效果。”主控给了收起的两种画法，用户：“甲乙都画出来，我挑”；看过样稿：“收起用7甲乙，展开用8二，写任务文件吧”；看过任务文件和主控另拿的五条主意：“五条都同意，派出去吧”。本轮只改 workspace 标题行和托盘怎么画；不改 workspace 的规则、菜单、浮层和存盘。
执行：派出
提示：沿用现有视觉和用语约定，聚焦指定的呈现结果。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`（规矩全文）。
- 样稿 `docs/设计稿/P5-91-workspace收起展开/`：README、`7-FoldNow`（现在的样子）、`7ab-FoldAB`（采用的收起）、`8-2-OpenWell`（采用的展开）。配色是近似值，实现时从主题取色；侧栏宽 340、卡片上的字和 agent 图标都是示意。
- `docs/DESIGN.md` §13 的 P5-89（workspace，托盘画法）、P5-76（分屏标签的叠卡）。
- 代码：
  - `app/src/sidebar_workspaces.rs` 的 `tray`（标题行和托盘现在怎么画）、开头的 `TRAY_*`、`TITLE`、`FOLDED`、`FACE`、`EDGE` 等常量、`bar_of`。
  - `app/src/card.rs` 的 `Tray` 和 `lines` 里往里填 `Tray` 的那一段。
  - `app/src/tab_stack.rs` 全文（叠卡的画法：`Face`、`Inks`、`cards`、`stack`），和 `app/src/window.rs` 的 `stack`（标签那边怎么从 agent 的种类凑出 `Face` 和 `Inks`）。
  - `app/src/sidebar.rs` 里把 `Line::Workspace` 交给 `tray` 的那一段，和按托盘宽度算卡片宽度的那一处（搜 `TRAY_X`）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-91-workspace-look`，分支 `p5-91-workspace-look`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-91-workspace-look`。
- 只动 `app/src/sidebar_workspaces.rs`、`app/src/card.rs`、`app/src/tab_stack.rs`、`app/src/sidebar.rs` 里和 workspace 标题行、托盘有关的地方，相关测试，`docs/DESIGN.md` 和本文件。标签那边凑 `Face`、`Inks` 的代码要给左侧栏共用时，可以挪到两边都够得着的地方，标签的样子不能变。

## 现在是怎样
workspace 的标题行前面是一个小箭头（收起朝右、展开朝下）和一个强调色的层叠图标，层叠图标两种状态一样。托盘的底是文字色的 3.5%、边 9%，收起和展开用同一套；收起就是一行标题加右边的状态细条。

## 要做的

### 收起（样稿 `7ab-FoldAB`）
- 收起的 workspace 那一行画成一摞卡片：
  - 最上面是一张实底的卡片（不透下面的东西，比侧栏底色亮一档，边比现在明显），里面照旧是箭头、图标、名字、个数、活动 agent 在里面时的小圈、右边的状态细条。
  - 下面露出别的卡片的下边：每往下一层，左右各缩进一点、往下错开一点、颜色更接近侧栏的底。里面 2 个 agent 露一层，3 个以上露两层，1 个或没有不露。
  - 露出来的层算这一行的高度，不能压住下面那一行。
- 标题前的层叠图标换成这个 workspace 里 agent 的小卡片叠起来，画法和分屏标签上的一样（`tab_stack`）：最多三张，最前面一张往左歪，后面的每张往右错开；卡片上是 agent 种类的图标和颜色。叠卡上不挂状态点。
  - 最前面一张：活动窗格正在显示的那个 agent（它在这个 workspace 里的话），否则是这个 workspace 在列表里的第一个 agent；后面的按列表里的先后。
  - 只有 1 个 agent 时画一张，照样歪着。
  - 没有 agent 的 workspace 收起时没有卡可叠：用层叠图标，顶上那片填实（和展开时的空心区别开）。
- 鼠标停在上面、菜单打开时照现在的做法变亮一点；`⋯` 按钮、点标题展开、就地改名都不变。

### 展开（样稿 `8-2-OpenWell`）
- 展开的托盘改成凹下去的槽：底比侧栏的底色深一档，边比现在明显。标题行还是箭头、层叠图标（空心）、名字、个数。
- 槽里的仓库小标题、卡片、`No agents` 那几行内容不变；卡片看起来和槽外面的卡片一样亮，不跟着槽变暗。
- 浅色主题下也要看得出是一个槽（比侧栏底色深）。
- 鼠标停在上面、菜单打开时的变化照现在的做法保留，只是在新的底和边上变。

### 别处
- 不在 workspace 里的仓库小标题、仓库的收起、左侧栏收成窄条、卡片本身：都不变。
- `docs/DESIGN.md` §13 在 P5-89 后面加一条 P5-91：用户原话、选的画法、上面定下的行为。

## 主控想加、问过用户的（10-14：“五条都同意，派出去吧”）
样稿上没有或看不出来，是主控的主意，用户五条都同意。
1. 叠卡最前面放活动窗格显示的那个 agent，没有就取列表里第一个。
2. 露几层按 agent 个数：2 个一层，3 个以上两层，1 个或没有不露。
3. 只有 1 个 agent 时也画一张歪着的卡；没有 agent 时用顶上填实的层叠图标。
4. 叠卡上不挂状态点（右边的细条已经说了状态）。
5. 槽里的卡片和外面的一样亮。

## 怎么算做完
- 用户原话：“展开和收缩的图标一样，我觉得应该改一下”“展开的情况下，workspace的展开状态（ui层面不明显，不好看出来这是一个展开的workspace）”“收缩情况下现在ui太普通了。能不能做一个类似之前agent区域的tab按钮的那个stack的效果”“收起用7甲乙，展开用8二”——上面「要做的」逐条达到，样子和样稿 `7ab-FoldAB`、`8-2-OpenWell` 一致。
- 一条测试：收起的 workspace 叠哪几张卡、谁在最前面（活动窗格的那个在前；没有时按列表先后；最多三张），用合成的 agent，照 `tab_stack` 和 `app/tests/workspaces.rs` 现成测试的写法。
- 验证只做这些：在 `app/` 下带上面的 `CARGO_TARGET_DIR` 跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`，再跑 `git diff --check`，各一次。觉得不够，在回复里说，不要自己加。
- 不起窗口、不截图。样子留给用户实际看：一摞的层次和间距、叠卡的大小和歪的角度、槽的深浅、浅色主题、界面字号调大和侧栏拖窄时的样子。

## 不要做
- 不改 workspace 的规则（谁归哪、名单怎么存）、菜单、浮层、文案，不改布局文件和 `workspaces.json` 的格式。
- 不改分屏标签上叠卡的样子，不改卡片和仓库小标题的画法。
- 不做：给 workspace 配颜色或图标、展开和收起的动画、点叠卡里某一张跳到那个 agent。
- 不调 `corral start`／`stop`／`send`／`keys`；不读 Corral 的内部状态目录；`corral ls` 里的都是用户的 agent，不碰。
- 不加依赖；不用 Python；不依赖 ratatui、crossterm；不起 paddock 窗口；不用脚本模拟按键或鼠标；不碰别的仓库。
- 命令里不用 `rm`，不用 `sh -c` 包长命令，前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。
- 不照抄 Zed 的 `terminal`、`ui` 等 GPL 代码；GPUI 本身的源码可以读来确认接口。
- 遇到要改本文件没列的模块才做得成、或者 GPUI 画不出上面的样子（比如露出来的层、歪着的卡），停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-91-workspace-look` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
