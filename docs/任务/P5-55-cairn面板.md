# 任务：右侧栏加第四个标签 Cairn：看 cairn 在不在工作、下次会话会接到什么，没采用的仓库可以点 Adopt

2026-10-10，paddock/main 交给 paddock/dev-cairn（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规 1.06；交叉审查拿不准，主控定不要：只在本机显示 cairn 自己的记录、不外发，唯一的写操作是用户确认后跑 `cairn adopt`，可用 `cairn unadopt` 撤回；影响面拿不准，主控定改行为）
类型：功能变更
依据：
- 用户 10-08：“paddock写cairn的面板”。10-10：“那就开始P5-55 这个任务，来确定cairn工作正常。咱们俩先确定一下scope。之后再委派”。
- 主控列了第一版范围（右侧栏第四个标签 `Cairn`，跟着当前窗格的仓库；状态行加下次会话会注入的全文；只用 `cairn status --json` 和 `cairn show --json`；只在标签显示时读；不做历史记录列表）。用户：“如果没接入cairn的情况你是不是要考虑，或者监测到cairn但是没adopt这种，是否提供adopt接入。但是不给安装cairn或者hook的选项。这个要讨论一下”。主控按状态给了建议（Adopt 要确认一步；hook 一家都没装、不在 Git 仓库时不给 Adopt；不给 Unadopt；没装 cairn 时标签照常显示一句话），用户：“都按你的建议来，出样稿吧。”
- 样稿（`docs/设计稿/P5-55-Cairn面板/`）出来后，用户问“hooks是不是不支持？”，主控答面板不装也不卸 hook；用户：“加一句提示命令的文字吧，然后写任务文件”。
依赖：P5-53
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- 样稿 `docs/设计稿/P5-55-Cairn面板/README.md` 和四个 `.dc.html`（读源文件里的尺寸、间距、文字；颜色是近似值，一律从主题取色）。
- `docs/DESIGN.md` §3（和外部命令打交道的规矩，表格里现在只有 corral 等）；§13 的 P5-29（Kanban：跟着当前窗格的仓库、只在显示时读）。
- `app/src/right_panel.rs` 全部（`Tab`、`Tab::ALL`、`label`、`empty`、`Saved`、`MIN_WIDTH`、`render` 里画标签的那一段）。
- `app/src/kanban_view.rs`：`Frame`、`frame`、后台读的写法（`cx.background_spawn` 读完回到视图）、`Clearing` 那一套“先确认再做”的写法、任务弹框 Preview 里画 Markdown 的那一段（`markdown::blocks`、`styled`）。`app/src/kanban.rs` 开头的说明（知道什么的模块和画的模块怎么分）。
- `app/src/window.rs`：`RightTab::Kanban` 出现的几处（建视图、订阅、`frame`、按标签选内容）。
- `app/src/command.rs`（跑一条短命令：超时、取消、输出有上限）、`app/src/git.rs` 的 `repository`、`app/src/markdown.rs`、`app/src/footer_icon.rs`（标签图标）。
- 假命令脚本的写法：`app/tests/pause.rs` 开头（假 `corral`）。
- cairn 这边只读、只作参考：`../cairn/docs/DESIGN.md` §7（命令接口，标着“草案”）、§9（注入文字长什么样）。不要改 cairn 仓库。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-55-cairn-tab`，分支 `p5-55-cairn-tab`（已从 main 建好）。
- 只动 `app/src/right_panel.rs`、`app/src/window.rs`、`app/src/footer_icon.rs`、`app/src/lib.rs`，在 `app/src/` 新加两个模块（知道什么的 `cairn.rs`、画的 `cairn_view.rs`），可以在 `app/tests/` 加一个测试文件，`docs/DESIGN.md` §3 和 §13，和本文件末尾的完成记录。`app/src/markdown.rs` 只有下面第 5 条的换行需要时才动。
- 编译目录：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-55-cairn-tab`（已备好，增量编译）。

## cairn 给什么（主控 10-10 在 cairn 0.1.0、`../cairn` 的 `08ce979` 上实测）
- `cairn status --json`，在某个目录里跑，只读、不改 cairn 的任何东西。用到的字段：
  - `agents.claude.installed`、`agents.codex.installed`（布尔）
  - `project.status`：`"adopted"` 或 `"not_adopted"`
  - `spool.pending_json`（数字：agent 存了、还没收进数据库的条数）
- `cairn show --json`，在某个目录里跑：`{"text": "…", "record_ids": [...], "omitted_sources": 0}`。`text` 是 Markdown，和下次会话会被注入的内容相同。
  - 还没有数据库时返回 `{"status":"no_data"}`。
  - **没采用的仓库里它照样返回一段抬头文字、退出码 0**，所以采没采用只看 `status`，不看 `show`。
  - 它有一个副作用：跑之前会把存了没收的记录收进 cairn 的数据库（cairn 的设计：用户命令执行前都先收取）。这是 cairn 自己允许的，不用绕开；只是别在没采用的仓库里跑它。
- `cairn adopt`，在仓库里任意目录跑，采用这个仓库；成功退出码 0。
- 命令名就是 PATH 上的 `cairn`，不加配置项。

## 要做的（照样稿）
1. **第四个标签 `Cairn`**：排在 Kanban 后面，带一个叠石小图标（三块由大到小叠起来的扁石头，`footer_icon.rs` 加一个）。右侧栏的最小宽度（`MIN_WIDTH`，现在按三个标签一行定的 320）跟着重定，四个标签仍在一行。旧的布局文件照常能读。
2. **跟着当前窗格的目录**（和 Kanban 一样从 `Frame` 拿 `cwd`）；只在右侧栏开着、且正显示这个标签时读：显示出来时读一次，窗格换了目录再读，之后每 5 秒读一次；切走或收起就停。命令在后台跑，不卡界面，用 `command::run` 带超时。
3. **读的顺序**：先 `status`；只有 `project.status` 是 `adopted` 才跑 `show`。
4. **各状态显示什么**（文字照样稿，英文）：

   | 状态 | 怎么判断 | 显示 |
   | --- | --- | --- |
   | 没装 cairn | 找不到 `cairn` 命令 | `cairn is not installed` 加一句说明；没有按钮 |
   | 命令出错或超时 | 退出码非 0、超时、JSON 读不出要用的字段 | `Could not read cairn` 加错误的第一行；没有按钮 |
   | hook 一家都没装 | 两个 `installed` 都是 false | 头部和状态行，`cairn's hooks are not installed`，一句说明，下面两行等宽字的命令文字 `cairn install --agent claude`、`cairn install --agent codex`（只是文字，不是按钮）；不给 Adopt |
   | 没采用，在 Git 仓库里 | `not_adopted`，`git::repository` 找得到仓库 | 一段说明、仓库路径、`Adopt…` 按钮 |
   | 没采用，不在 Git 仓库里 | `not_adopted`，找不到仓库 | `This folder has not adopted cairn` 加一句 `Adopt is offered inside Git repositories only.`；没有按钮 |
   | 已采用，还没记录 | `record_ids` 为空，或 `no_data` | 头部和状态行，`No records yet` 加一句说明 |
   | 已采用，有记录 | 其余 | 头部、状态行、正文 |

   - 头部：仓库名（不在仓库里就是目录名）、分支（有才画）、右边一颗 `Adopted`（绿）或 `Not adopted`（灰）。
   - 状态行：`Hooks`，`Claude`、`Codex` 各带一个勾（装了）或一道短横（没装，变淡）；已采用时右边 `N uncollected`，0 时很淡，不是 0 时黄色小胶囊。
5. **正文**：`text` 从第一个以 `### ` 开头的行起显示（前面两段是写给 agent 的约定说明）；找不到这样的行就全文显示。用现有的 `markdown::blocks` 画，能上下滚动：
   - cairn 的标题层级是反的：`###` 是大节（本工作线、现场对比），`##` 是记录里的小节（停点、已完成及验证……）。把三级标题画成淡的分节小标题加一条细线，二级标题画成加粗小标题（见样稿）。
   - “现场对比”那一段靠一行一条事实，中间没有空行：软换行要按换行画。`markdown::blocks` 现在把软换行并成空格（Kanban 的 Preview 靠它），别改它的默认行为；需要的话加一个不影响 Kanban 的开关。
   - 记录编号、来源这类很长的词能在任意处折行，不撑出面板。
6. **Adopt**：点 `Adopt…`，说明那一块换成确认卡片（样稿 `Confirm.dc.html`：标题带仓库名、仓库路径、立即生效的那段话、派出去的 agent 不受影响和怎么撤回、`Cancel`／`Adopt`）。点 `Adopt` 才在这个窗格的目录里跑 `cairn adopt`（后台），跑完马上重读；失败就在卡片里显示错误的第一行，卡片留着。点 `Cancel`、窗格换了仓库、或切走标签，确认卡片收起。
7. **DESIGN**：§3 的表格加一行 cairn（归属：独立仓库 `../cairn`；paddock 只调用 `cairn status --json`、`cairn show --json`、`cairn adopt` 三条，不读它的数据库、配置和 hook 文件，不装不卸 hook；cairn 的命令接口在它自己的设计里还标着“草案”，格式变了 paddock 跟进）；§13 加 P5-55 一条（照上面和依据里的用户原话）。

## 怎么算做完
- 用户原话：“paddock写cairn的面板”“那就开始P5-55 这个任务，来确定cairn工作正常。”“如果没接入cairn的情况你是不是要考虑，或者监测到cairn但是没adopt这种，是否提供adopt接入。但是不给安装cairn或者hook的选项。”“都按你的建议来”“加一句提示命令的文字吧”。
- 测试先看它失败再实现，用假 `cairn` 脚本和合成的 JSON，不碰真的 cairn：从两条命令的输出判断出上表七种状态；没采用时不跑 `show`；正文从第一个 `### ` 行起、找不到就全文；Adopt 只在确认后跑、跑的目录对、失败时带回错误的第一行；`Tab` 多了一个之后旧布局文件照常读、最小宽度按四个标签算。
- 验证只做这些：上面的测试；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 样子留给用户看。

## 不要做
- **不要起 paddock 窗口**（`cargo run`、截图、实测都不要）：在 agent 会话里起的测试窗口会和所有 agent 归进同一个进程组，从 Dock 上退出它会把全部 agent 一起结束（见 `docs/任务/P5-52-Dock里第二个paddock进程.md`）。
- **不要跑真的 `cairn adopt`、`cairn unadopt`、`cairn install`、`cairn uninstall`、`cairn save`、`cairn delete`**，也不要读 `~/.local/state/cairn/` 和暂存目录里的东西。要看真实输出的样子，只许在本仓库里跑 `cairn status --json`（`cairn show` 会收取记录，别跑；它的输出样子见上文和 `../cairn/docs/DESIGN.md` §9）。
- **不用 Python**（命令、脚本、测试都不用，`python3 --version` 也不要跑）；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带会等标准输入的东西。
- 不做：历史记录列表、点开单条记录、Unadopt、更正／撤回／删除记录、安装或卸载 cairn 和 hook 的按钮、手动刷新按钮、设置项、`cairn` 路径的配置项。不解析 `cairn list` 和 `cairn show <ID>` 的文字。不解析正文里的内容（只按 Markdown 画）。
- Changes、Browser、Kanban 三个标签的行为不改。不加新依赖。
- 不复制 Zed 的 GPL 代码（`terminal`、`terminal_view`、`ui` 等）；GPUI 本身可以读源码确认接口。
- 不要按项目名或路径批量杀进程；不要对 `corral ls` 里的 agent 做 stop、send、keys。
- 遇到要改上面“只动”以外的文件、或 cairn 的输出和上文写的对不上，停下来报告，等决定，不要自己换别的办法。
- 不合并到 main，不推送。只在 `p5-55-cairn-tab` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

2026-10-10，paddock/dev-cairn。

### 做了什么
- 右侧栏第四个标签 `Cairn`（`right_panel.rs`），叠石图标（`footer_icon.rs` 的 `Icon::Cairn`：三块由大到小的扁椭圆）。
- `app/src/cairn.rs`（知道什么）：`read` 先跑 `cairn status --json`，只有已采用且至少一家装了 hook 才跑 `cairn show --json`；仓库名、分支用只读的 git 命令（`git::repository`、`symbolic-ref`）。`shown` 取正文（第一个 `### ` 行起，找不到就全文）。`adopt` 跑 `cairn adopt`。`Adopting` 是“先确认再做”的那几步（问、取消、确认、跑完、重读后收起）。
- `app/src/cairn_view.rs`（画）：照四张样稿画七种状态、头部、状态行、确认卡片、正文；标签显示时读一次，窗格换目录再读，之后每 5 秒读一次，切走或收起就停并取消在跑的命令；命令都在后台跑。
- `markdown.rs` 加 `blocks_by_line`（段落和列表项里的换行照换行留着），`blocks` 的行为没变，Kanban 的 Preview 不受影响。
- `window.rs`：建视图、每帧把当前窗格的目录和“是否正显示”告诉它、按标签选内容。
- `docs/DESIGN.md`：§3 表格加 cairn 一行，§13 加 P5-55 一条。

### 验证了什么
- 测试都是先看它失败再实现的（先写空壳让它能编译，断言失败后再填实现；`no_data` 那条也是先失败）。
- `app/tests/cairn.rs`（假 `cairn` 脚本加合成 JSON，不碰真的 cairn）：从两条命令的输出判断出七种状态；`no_data` 当作没采用；没采用、hook 都没装时不跑 `show`；`status` 在前 `show` 在后、都在窗格目录里跑；worktree 里仓库名和路径是主仓库的、分支是自己的；正文从第一个 `### ` 行起、找不到就全文；Adopt 没问过不跑、确认一次只跑一次、在窗格目录里跑、失败带回 `cairn adopt: ` 加错误第一行且卡片留着、换仓库或切走后收起；读的过程中只出现 `status`、`show` 两条命令。
- `right_panel.rs` 的单元测试：四种 `tab` 值的旧布局文件照常读、写回原样；比新最小宽度窄的旧宽度原样留着、按最小宽度画；最小宽度按四个标签加两个按钮一行算。
- `markdown.rs` 的单元测试：换行并成空格（原行为）和照换行留着两种。
- `app/` 下各跑一次：`cargo test --all-targets`（541 项全过，原来 533）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check`，都干净。
- 没起窗口，样子没看过，留给用户看。

### 拿主意的地方
- **`project.status` 的第三个值 `no_data`**：任务文件只写了 `adopted`／`not_adopted`，cairn 源码（`crates/cairn/src/status.rs`）在还没有数据库时返回 `no_data`。停下来问了主控，主控定：当作没采用，和 `not_adopted` 一样处理。已照做并补了测试。
- **最小宽度 426**（主控定：按量的定，默认宽度 420 不动、不改 `layout_state.rs`）。怎么量的：在临时目录写了一个小 Rust 程序（不进仓库），用 CoreText 取系统界面字体 12.5pt，量四个标签的字宽：Changes 51.54、Browser 47.62、Kanban 43.95、Cairn 30.93（粗体分别是 54.78、51.60、46.93、33.38；选中的那个是半粗，按最多多 4pt 算）。一行要的宽度 = 左边距 10 + 四个标签（每个：左右各 11、图标 12、图标后 7、字宽）+ 标签间 2×3 + 选中加粗 4 + 间隔 2 + 伸缩空位 + 间隔 2 + 按钮 26 + 间隔 2 + 按钮 26 + 右边距 8 = 424.04，取 426。这些数写在 `right_panel.rs` 的测试里。
  - 顺带量出来：三个标签时一行要约 350，原来的 320 是只有两个标签时定的，之后没跟着改过。
  - 默认宽度 420 比 426 小，所以首次打开实际是 426；`right_panel.rs` 里三条原先默认“420 在范围内”的测试改成用 500 起步，另加一条断言写明默认宽度按最小宽度画。窗口很窄时右侧栏保最小宽度，终端会比以前少 106pt。
- hook 一家都没装时不跑 `show`（它的结果这时不显示，少一次收取的副作用）；这时头部照样按 `status` 显示 `Adopted` 或 `Not adopted`。
- 仓库名和 Adopt 卡片上的路径取主工作区（公共 `.git` 目录的上一级），所以在 worktree 窗格里显示的是仓库本身，和 cairn 按整个仓库算采用一致；公共目录不叫 `.git`（裸仓库、子模块）时用工作区自己的目录。分支是窗格所在工作区的。
- 错误那一行写成 `cairn <命令>: ` 加 stderr 的第一行（没有就取 stdout 的，再没有写退出状态）；超时写 `timed out`。cairn 自己的 stderr 不带前缀，所以和样稿的 `cairn status: database is locked` 一致。
- `cairn adopt` 成功后，卡片先写 `Adopted`，等重读回来才换成已采用的样子（照 Kanban 的 `Done`），免得中间闪回 `Adopt…` 按钮。失败后卡片上 `Cancel`、`Adopt` 都还在，可以再试。
- 样稿没画的两种：没有窗格在焦点上写 `No pane in focus`，窗格没有目录写 `No directory to read`，下面都是 `Cairn follows the focused pane's directory`（照 Kanban 的写法）。窗格换目录时，旧内容留到新的一次读回来（约零点几秒），和 Kanban 一样。
- 正文里画 Markdown 行内样式的那个函数（`styled`）在 `kanban_view.rs` 里是私有的，那个文件不在“只动”清单里，所以在 `cairn_view.rs` 里另写了一份同样的。
- 样稿里“本工作线”下第一段（记录编号那一行）比正文小一号、淡一点：要认出这一段就得解析正文内容，没做，所有段落一个样。分节小标题的字距（样稿 0.04em）也没做。
- 三级以外的标题（一、二、四级以上）都画成加粗小标题。

### 没做的事
- 任务文件“不做”里列的都没做：历史记录列表、点开单条记录、Unadopt、更正／撤回／删除记录、安装或卸载 cairn 和 hook 的按钮、手动刷新、设置项、`cairn` 路径配置。
- 没起 paddock 窗口，没截图，没跑真的 `cairn adopt`／`show`（只在本仓库里跑过一次 `cairn status --json` 看输出的样子）。
- 没合并，没推送。

### 留给用户看的
- 把右侧栏拖到最窄，四个标签和右边两个按钮是不是正好一行；界面字号调大后也看一眼。
- 七种状态的样子、正文的分节和折行、Adopt 的确认卡片（真点 `Adopt` 会采用那个仓库，可用 `cairn unadopt` 撤回）。
