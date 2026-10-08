# 任务：Settings 重新设计（精修的独立窗口），现有功能一项不丢

2026-10-08，paddock/main 交给 paddock/dev-settings（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（常规 0.84），交叉审查不要，影响面改行为；档按规则定常规）
类型：功能变更
依据：`docs/DESIGN.md` §13 最后一条（P5-44）；样稿 `docs/设计稿/P5-44-Settings再设计/` 的 A 系列。仍是独立窗口；不改配置文件格式和保存规则（`settings.rs` 的 Draft／Saved／Conflict 照旧）。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」。
- `docs/DESIGN.md` §13 的 P3-1（Settings）、P4-2（界面字体）、P5-40（主题选择）和最后一条 P5-44；P5-4 弹出界面的样式约定。
- 样稿：`docs/设计稿/P5-44-Settings再设计/` 的 `A-General`、`A-Appearance`、`A-Agents`、`A-Diagnostics`（`B-Sheet` 未采用），README 说明取舍。
- 代码：`app/src/settings_view.rs`（整个窗口）、`app/src/settings.rs`（页面、字段、Draft、保存和冲突）、`app/src/search.rs`（命令面板里列出的设置页和说明）、`app/src/windows.rs`（`open_settings`）、`app/src/menu.rs`（⌘S／⌘W）、`app/src/diagnostics.rs`、`app/src/text_input.rs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-44-settings`，分支 `p5-44-settings`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-44-settings`。

## 要做的
用户 10-08：“settings对话框也有这个问题。你也要重新设计一下，但是不要丢失任何现有的功能”。主控截图指出旧感的来源（每行带外框的卡片和通栏分隔线、又大又方的右对齐输入框、全大写分组标题、Colors 页一长串原始键名、Advanced 整页一行、常驻底栏），出 A（精修的独立窗口）、B（主窗口浮层）两种样稿，推荐 A，并建议：Colors 改名 Appearance、Advanced 并入新的 Agents 页（Refresh interval 一起挪过去）、Terminal 加实时预览、Revert／Save 改成只在有改动时浮出的保存条。用户：“选A，按你的建议，写任务文件吧。”

**1. 页面**（`settings.rs` 的 `Page` 和字段归属）
- 四页：General、Appearance（原 Colors）、Agents（原 Advanced）、Diagnostics。Refresh interval 从 General 挪到 Agents，和 corral command 一组。General 剩 Interface、Terminal、Mascot 三组。
- 命令面板（`search.rs`）里设置页的名字和说明跟着改（Appearance：theme and colors；Agents：refresh interval, corral command）。

**2. 照样稿 A 的样子**（只从主题取色；界面字号 13 和 18 都放得下；窗口仍可调大小，内容区可滚动）
- 左侧栏：四页带线条图标，选中有底色；配置文件路径放左侧栏底部（等宽小字）。
- 页面：标题在上；分组标题不再全大写；行放在浅底卡片上，卡片不加外框，分隔线从标签处开始、不通栏。
- 数字（界面字号、侧栏宽度、终端字号、行高、刷新间隔）改成“− 值 单位 +”步进器，中间仍可直接输入；步进的步长你按每项的合理值定，写进完成记录。
- Fallback fonts 改成一个个可删的标签加 “+ Add”（加的时候用现有的输入框打名字；保存出来仍是同一个列表）。
- Terminal 组上方加实时预览：用当前草稿里的终端字体、字号、行高和主题颜色画两三行终端文字；改了这几项就跟着变（不用先保存）。
- 改过的行（和默认值不同）：标签前一个强调色小圆点，标签旁 “Reset to <默认值>”，点了回默认（取代现在的 `custom` 标记和 Default 按钮）；要重启才生效的行标黄色 “after restart”（取代 `Restart required`）。
- Appearance：主题改成缩略窗口卡片（每张用该主题的底色、侧栏色、强调色和几个状态色画一个小窗口，下面是名字，选中的一圈强调色并打勾），一行放得下就四张。自定义颜色：默认只列出改过的（色块、键名、所属分组、可输入的值框、Reset），右上 “Show all <总数>” 展开全部，按现有五组（Interface、Agents panel、Status、Agent types、Terminal）列出，每项色块＋键名＋值框；没有改过的时候默认就显示 Show all 的入口。
- Agents：Refresh interval、corral command 两行，各带 “after restart”；corral command 下面一行说明 “--corral overrides it for one run”。
- Diagnostics：五组（Commands、Agents、Config、Layout、Start）排成两栏，每行前一个状态点（正常、未知、出错三种颜色，沿用现在 Tone 的含义），Refresh 改成标题右边的图标按钮，旁边写检查时间和 read-only。
- 保存条：底部居中浮出，只在草稿和文件不同时出现：小圆点、“N unsaved change(s) · <改了哪几项>”、Revert、Save ⌘S。保存后的消息（Saved.／Saved. Restart paddock for: …／Not saved: …／Nothing to save.）、磁盘冲突（The config file changed on disk; nothing was saved. ＋ Keep my edits／Discard my edits）、配置文件有问题的提示，都显示在这个位置（消息几秒后淡出或下次编辑时消失，你定；冲突和错误要用户处理才消失）。去掉常驻底栏。

**3. 现有功能一项不丢**（逐项核对，完成记录里写对照）
- 全部字段：Interface（字体、字号、侧栏宽度）、Terminal（字体、字号、行高、Fallback fonts 及其说明）、Mascot（开关、Pet 三选一，关掉时 Pet 变淡）、Refresh interval、corral command、Theme、全部颜色覆盖（值可写颜色名或 #hex，空即跟随主题）。
- 字体选择框：搜索、上下键选、Return 选定、Esc 收起、加载中／没有匹配的提示、当前字体打勾、界面字体列 System 项。
- 单位（pt、ms）、输入不合法时的红框和原因、过窄的 sidebar_width 自动加宽（主窗口那边）。
- Save（⌘S）、Revert、⌘W 关闭；带着未保存改动关闭时问 Save／Don't Save／Cancel；保存写临时文件再替换；磁盘冲突的 Keep／Discard；配置文件读不了时的提示和退回默认；保存后主窗口立即应用（`SettingsEvent::Saved`）。
- Diagnostics 的全部行和文字、checking… 状态、Refresh、“主窗口不在”时的提示。
- 命令面板能跳到各设置页。

## 怎么算做完
- 用户原话：“settings对话框也有这个问题。你也要重新设计一下，但是不要丢失任何现有的功能”“选A，按你的建议，写任务文件吧。”
- 页面归属改动（Refresh interval 进 Agents、页名改了）有测试；`settings.rs` 原有的 Draft／保存／冲突测试照旧通过。
- `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。
- 截图（只截自己的测试窗口，临时 HOME／XDG、假 corral、`PADDOCK_NO_ACTIVATE=1`；需要临时开关让启动后自动打开 Settings、切到某页、造出未保存改动的，截完撤掉、不提交）：四个页面各一张、有未保存改动时的保存条、展开 Show all、界面字号 18 的 General。截图放 scratchpad，不入库。
- 完成记录里逐项写上面第 3 条的对照（每项在新界面的哪里）。
- 键盘、鼠标、字体选择框、步进器、关闭确认这些交互，留给用户在真窗口里试。

## 不要做
- 不改配置文件格式、键名、默认值和保存规则；不改 `settings.rs` 里 Draft／Saved／Conflict 的逻辑（只改页面归属和显示需要的小处）。
- 不把 Settings 改成浮层（那是未采用的 B）。
- 不加新依赖；不用 Python。
- 命令里不用 `rm`，不用 `sh -c` 包长命令。
- 不合并到 main，不推送。只在 `p5-44-settings` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
