# 任务：New Agent 对话框重新设计（先写任务＋预设）；种类图标用本机的真实原图

2026-10-07，paddock/main 交给 paddock/dev-newagent（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：三项都拿不准；界面重做加图标来源，按规则用常规；会决定开出来的 agent 用什么模型和强度，定改行为；不碰数据和并发，定不交叉审查）
类型：功能变更
依据：本轮重做 New Agent 窗口，并让种类图标优先用本机的彩色原图；开 agent 仍走 `corral start`，标签规则照 P5-21。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」。
- `docs/DESIGN.md` §13 最后两条（New Agent 重新设计、种类图标用真实原图）和 P5-21（New Agent 记下模型和强度）、P5-4／P5-10 弹出界面的样式约定。
- 样稿：`docs/设计稿/P5-37-NewAgent/` 的 `B-Composer.dc.html`（主体）、`C-Presets.dc.html`（取其预设），README 说明取舍。
- 代码：`app/src/new_agent_view.rs`（现在的窗口）、`app/src/new_agent.rs`（拼命令、认模型和强度、标签；P5-36b 刚修过边角）、`app/src/kind_icon.rs`（单色剪影）、`app/assets/kinds/README.md`、`app/src/config.rs`（配置读写）、`app/src/text_input.rs`（单行输入；刚加了撤销）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-37-newagent`，分支 `p5-37-newagent`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-37-newagent`。
- 另有两件活同时在做：P5-35 在改 `layout.rs`、`layout_state.rs`、`window.rs` 的窗格绘制；P5-36a 在改 `sidebar.rs`、`kanban_view.rs`、`about.rs`、`config.rs`（侧栏宽度那一处）。图标的改动尽量收在 `kind_icon.rs` 里、保持它对外的用法不变，这样卡片、窄条、标签、Kanban 不用改调用处；确实要改这些文件，只改最小的一处，`config.rs` 只加你需要的字段。

## 要做的
用户 10-07：“new agent那个对话框再设计一下，现在的太初级了。”看过三张样稿，主控推荐“B 为主体＋C 的预设”，用户：“可以。同意你的建议。”图标：“现在的claude，codex，pi，omp的图标都是模拟的，我觉得直接上真实的吧。我自己用的。暂时不会分发。”主控指出仓库公开，建议图标只放本机，用户：“图标我同意你的做法”。

**New Agent 窗口**
1. **大输入框在最前**：“What should it work on?”，多行，可留空（留空就不带第一句话、开出来空闲）；打开窗口时焦点就在这里；⌘↩ 创建，Esc 取消。输入框沿用（或扩展）现有的文本输入，保留撤销和输入法。
2. **一排可点开的选项**，在输入框底边：项目、种类、模型、强度、打开位置、角色。点一个弹出它的选项列表，每项一句说明（照样稿）；再点或选完收起。
   - 项目：常用项目（现在开着的 agent 的项目、最近用过的）＋ Other…（照旧弹出选目录）。
   - 种类：Claude／Codex／pi／omp，带种类图标。
   - 模型、强度：Claude 和 Codex 有，pi、omp 没有（隐藏这两项，用它们自己的默认）。可选的值：Claude 模型 `opus[1m]`、`opus`、`sonnet`，强度 `low`／`medium`／`high`／`xhigh`／`max`；Codex 模型 `gpt-6-astra`、`gpt-5.6-luna`，强度 `low`／`medium`／`high`／`xhigh`。选了就按 P5-21 的写法拼进命令（Claude `--model`／`--effort`，Codex `-m`／`-c model_reasoning_effort="…"`），标签照旧自动加。
   - 打开位置：Split right／New tab／This pane（沿用现在的分屏方向选择）。角色：Regular／Controller（照现在的含义）。
3. **预设**：输入框上方一排预设按钮，点一个就把种类、模型、强度、角色一次填好（不碰项目和输入框里的字）。默认四个：Controller（Claude、opus[1m]、xhigh、Controller）、Developer（Claude、opus[1m]、high）、Codex builder（Codex、gpt-6-astra、high）、Quick look（Claude、sonnet、medium）。末尾一个 “+” 把当前设置存成新预设（起个名字），预设存在配置文件里，可以删。
4. **名字**：自动起（照现在的规则），显示在输入框上方，点一下可改。
5. **命令**：原始的 `corral start …` 不再常驻，收进 “Show command”（点开看全文，可以照旧在里面手改命令；手改后上面的选项按命令里认出的值同步，认不出的不动）。
6. 窗口样式沿用 P5-4 弹出界面那套（颜色、圆角、按钮），只从主题取色；界面字号 13 和 18 都放得下；改了字体或界面字号，开着的窗口立刻跟着变（包括命令全文）。

**种类图标用本机的真实原图**
7. 本机 `~/.config/paddock/icons/` 里有 `claude`、`codex`、`pi`、`omp` 的 `.png` 或 `.svg`（同名都有时用 svg）时，按原色显示原图，不再染成主题色；没有、读不了或格式不对，就退回仓库 `app/assets/kinds/` 的单色剪影。卡片、窄条、标签、Kanban、New Agent 处处一致。主控已放好四个文件（`claude.png`、`codex.png` 是带透明边的应用图标，`pi.svg`、`omp.png`／`omp.svg`），看看小尺寸下透明边是否要裁掉、各家视觉大小是否要像现在这样调，按好看来定。
8. 图标文件不进仓库，代码里只写这个目录；README（`app/assets/kinds/README.md`）补一句本机原图的约定。读图在启动时或第一次用时做一次，不每帧读盘。

## 怎么算做完
- 上面八条达到，和样稿 B（加 C 的预设）一致或更好。
- 验证只做这些：
  - 测试：预设填值（不碰项目和输入）、模型和强度拼进命令和标签（两种 agent 各一例，pi／omp 不加）、手改命令后选项同步、预设存读删（旧配置没有预设时用默认四个）、图标选择（svg 优先、缺文件退回剪影、坏文件退回剪影）。
  - 在 `app/` 下跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
  - 截图：New Agent 窗口一张（有一个选项列表展开更好；展开截不到就照 P5-29f 临时加一个只在截图时用的开关，截完撤掉不提交），左侧栏一张（看彩色原图在卡片和窄条上的样子）。release 程序、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／`XDG_CONFIG_HOME`／`XDG_STATE_HOME`、假 corral；临时 HOME 下自己放一份图标（从 `~/.config/paddock/icons/` 复制，只读不改那个目录）。自己看过。截图放 scratchpad，不入库。
  - 真实的点击、⌘↩、弹出列表留给用户试，写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不把任何图标文件提交进仓库；不改、不删 `~/.config/paddock/icons/` 里的文件（截图时复制到临时 HOME 用）。
- 不改 `corral start` 的调用方式和 P5-21 的标签规则；不改 Kanban 的 New task、Settings 窗口（除非为了预设的存读要加配置字段）。不新增主题颜色键，不加依赖（GPUI 自带的图片加载够用；不够时停下来报告）。
- 不要用 `osascript`、System Events 等任何方式模拟按键、鼠标或拖动。截图只截自己开的窗口，启动时一定加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral，不碰用户真实的配置、布局文件和正在运行的 paddock；测试里绝不真的开 agent。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys/pause，不 attach 上去打字。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。不用 Python。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-37-newagent` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

**做了什么**
- New Agent 窗口（`new_agent_view.rs`）按样稿 B 重做：最上面一排预设（Controller、Developer、Codex builder、Quick look，当前设置对得上的那个点亮；悬停出 × 可删；末尾 `+` 起名后把当前种类、模型、强度、角色存成预设，同名覆盖），下面是名字（`前缀/名字`，点一下变成可改的前缀、名字两个框，Return 或 Done 收起），再下面是大输入框（多行、可留空、打开就有焦点），输入框底边一排选项：项目、种类、模型、强度、打开位置、角色，点开是带勾和一句说明的列表（P5-4 的 popover 样式，挂在选项下面，空间不够往上开；点外面、再点一次或 Esc 收起）。底下一行是“Opens as …”、Show command、Cancel、Create（⌘↩）。Show command 里是可手改的命令（多行折行显示）和完整的 `corral start …`。
- 项目列表：现在开着的 agent 的项目、paddock 自己的目录，再加最近用过的（成功创建后记在 `$XDG_STATE_HOME/paddock/recent-projects.json`，最多 8 个），末尾 Other… 弹系统选目录。
- 种类加了 pi、omp（命令 `pi`、`omp`），这两种隐藏模型和强度。模型和强度按 P5-21 的写法写进命令（Claude `--model`／`--effort`，Codex `-m`／`-c model_reasoning_effort="…"`），只替换命令里原来问模型、强度的那几个词，其余（`--yolo`、手加的参数、`--` 后面的）原样保留；标签照旧从命令认（认法抽成 `new_agent::read`，行为和 P5-21／P5-36b 一致，原有测试全过）。手改命令后：认出的种类、模型、强度同步到选项，认不出的不动。
- 预设存在 `config.toml` 的 `agent_presets`（`Config` 只加了这一个字段）；没写时用默认四个，写了空列表就是空。保存用 toml_edit，其余内容和注释原样保留，写前用 `Config::parse` 校验。
- 单行输入 `text_input.rs` 加了折行多行模式（`wrapping(newlines)`）：上下键、Shift 选、行首行尾、Return 换行、粘贴保留换行；撤销／重做、输入法（候选框位置）沿用原来的那套。单行的行为不变。
- Esc：先收起小输入框或列表，没有打开的才关窗口（menu.rs 加了 New Agent 窗口里的 escape、小输入框里的 enter 两条绑定）。设置保存后，开着的 New Agent 窗口跟着换主题、终端字体和预设（`windows.rs` 转发一处）；界面字体和字号本来就跟全局走。
- 种类图标（`kind_icon.rs`，对外用法 `of(kind)`／`render(height, color)` 不变，卡片、窄条、标签、Kanban 没改）：第一次用时读一次 `~/.config/paddock/icons/`，svg 优先、再 png；读出来包进一个 SVG（png 用 data URI），量出不透明区域把透明边和阴影裁掉，之后按实际像素大小用 GPUI 自带的 resvg 栅格化、按尺寸缓存。读不了、格式不对或什么都不画，就退回仓库里的单色剪影。README 补了本机原图的约定。没加依赖。
- `window.rs` 只改一处：给 New Agent 的 Seed 带上现有 agent 的名字（用来起不重名的名字）。

**验证了什么**
- 新测试：预设填值不碰项目、前缀、打开位置和输入；Claude、Codex 各一例模型和强度进命令和标签，pi／omp 不加；选项只替换命令里对应的词；手改命令后选项同步（含换种类、认不出不动）；名字跟种类取空号、改过就不跟；预设存、读、删（无文件／无预设时默认四个，删光后保持空，其余配置和注释保留，坏种类报出名字）；最近项目；图标 svg 优先、只有 png 也行且裁掉透明边、缺文件／坏 svg／坏 png／空图退回剪影、坏 svg 旁边有好 png 用 png；Base64；多行粘贴保留换行且一步撤销。
- `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 全过。
- 截图（release、`PADDOCK_NO_ACTIVATE=1`、临时 HOME／XDG、假 corral、临时 HOME 里复制的图标；临时开关截完已撤掉没提交）：New Agent 展开模型列表、展开打开位置列表、界面字号 18 下展开种类列表；左侧栏展开时的卡片和收起后的窄条。都自己看过：彩色原图在卡片和窄条上清楚、大小相当，没有裁不干净的透明边；字号 13 和 18 下选项一行放得下。

**拿主意的地方**
- 打开时默认是 Claude、opus[1m]、high、Regular（和样稿 B 一致）；Form 本身的默认（Codex 主控）没动，老测试照旧。
- 名字：前缀照旧跟项目；主控照旧是 `main`；普通 agent 原来默认填 `main`，现在改成“种类-第一个空号”（`claude-1`、已有就 `claude-2`），改过就不再跟。
- 手改命令换了种类（比如 claude 改成 codex）时，模型和强度先清空再按命令认，免得 Codex 下挂着 Claude 的模型；同一种类内认不出的保持不动。
- 预设按钮只显示图标和名字（试过加模型·强度的小字，两行放不下、显得乱）；是否是当前设置靠点亮。Create 从输入框底边挪到最下面一行和 Cancel 并排，让六个选项一行放得下。窗口 640×600（随界面字号放大）。
- 原图不再染主题色；视觉大小没再按种类调（裁掉透明边后四个看起来已经相当）。

**没做的事**
- 真实的点击、⌘↩ 创建、弹出列表的鼠标操作、输入法输入、改名、存删预设、Show command 里手改命令，留给用户实际试。
- 长文本超出输入框最大高度后能滚动，但光标移出可视范围时不会自动滚过去。
- 列表没有键盘上下选择（只能鼠标点）。

## 主控审查

- 八条都达到，和样稿 B 加 C 的预设一致；没有任何图片进仓库（diff 里没有 png／svg／icns）。看过截图：New Agent 预设一排、大输入框、底边六个选项、模型列表带说明；左侧栏四种 agent 显示彩色原图、大小相当。主控在分支上重跑 `cargo test --all-targets`（348 项全过）、clippy、fmt，都过；合并后在 main 上再跑全套。
- 对方请主控定的两处：普通 agent 默认名改成种类加空号（`claude-1`），主控仍叫 `main`——同意；手改命令换了种类时先清空模型和强度再按命令认——同意。其余取舍（Create 挪到底行、预设按钮不写小字、默认 Claude／opus[1m]／high）同意。
- 建议改未排：多行输入框光标移出可见范围时不自动滚过去；弹出列表不能用键盘上下选。
- 真实点击、⌘↩、列表、输入法、改名、存删预设、手改命令留给用户试。
