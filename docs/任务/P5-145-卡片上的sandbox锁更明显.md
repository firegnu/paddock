# 任务：卡片上的 sandbox 锁更明显

依据：用户 10-19 截了左侧栏的图（`paddock/dev-terminal-core` 那张卡，箭头指着强度 `high` 后面的小锁），说：“再加一个任务，这个锁能不能明显一点？”主控出了样稿（sandbox 画布 F 块，`docs/设计稿/P5-120-sandbox界面/Lock.dc.html`：现在、A 大锁、B 小胶囊、C 压在图标角上），用户：“选B，展开卡片和New Agent的锁都统一”；看过本文件后：“都同意，一起换，你自己做吧”。
执行：主控
（理由：只改界面上画锁的三处和一个主题颜色，不碰 sandbox、fence、corral 的逻辑；样稿是主控画的，代码位置已核对；量小，派出去的来回比自己做还长。主控不起窗口，样子留给用户实际看。）
待用户：装好后实际看（收起和展开的卡片、New Agent 底边、Blocked 浮窗、Settings › Sandbox：胶囊一眼看得出、不刺眼，和主控的 Delegates 标记分得开；侧栏拖窄时名字那一行不乱）

## 现在是怎样（主控 10-19 对着代码核对的）
- 收起的卡片：名字、强度后面一把 12 点的灰色描边锁（`app/src/sidebar.rs` 约 3317 行，`LOCK = 12.0`，`footer_icon::lock(false, agents_dim, None, …)`），被墙拦过时后面一个黄色数字（`card.rs` 的 `Sandboxed::count`：`2`，只读了最近一页记录时 `2+`）。悬停 `Sandbox: dev · 2+ blocked`。
- 展开的卡片：`SANDBOX` 一格（`sidebar_sandbox.rs` 约 160 行）里同样的灰锁，后面 `dev · 3 blocked ›`。
- New Agent 底边：选了档案后是一颗蓝色（`agents_accent`）胶囊，锁加档案名（`new_agent_view.rs` 约 1600 行）；档案菜单里选中那一行的锁也是蓝色（约 1760 行）。检查没过时变红，有警告时右上角黄点。
- P5-141 主控卡片的 Delegates 标记是蓝色“箭头指向锁”（`sidebar_delegates.rs`），用的也是 `agents_accent`。

## 要做的（照样稿 B）
1. **一个 sandbox 专用颜色**：主题里加一个 `agents_sandbox`（青绿色），每套主题各配一个，要和这套主题的 `agents_accent`（蓝）、`agents_yellow` 分得开，浅色主题里在白底上看得清。只在“这个 agent 自己在墙里”的地方用；Delegates 标记仍用蓝色，不改。
2. **收起的卡片**：灰锁换成小胶囊：淡青绿底、圆角，里面是实心青绿小锁、档案名（青绿、半粗），被拦过时再接黄色 `· 2+`；没拦过只有锁和档案名。胶囊不被挤缩；名字一行放不下时，照现在的顺序让位。
3. **悬停文字**把 `+` 说清：只读了最近一页记录时写 `Sandbox: dev · 2 blocked in its latest records`；读全了写 `Sandbox: dev · 2 blocked`；没拦过照旧 `Sandbox: dev`。
4. **展开的卡片**：`SANDBOX` 一格里的锁和档案名换成同样的胶囊，后面 `· 3 blocked ›` 等照旧。
5. **New Agent**：底边的 sandbox 胶囊、档案菜单里选中行的锁，从蓝色换成 `agents_sandbox`；检查没过的红色、警告的黄点照旧。
6. **Denials 弹框和 Settings › Sandbox**：弹框标题里的锁（`sidebar_sandbox.rs` 约 274 行）和档案列表里的锁（`settings_sandbox.rs` 约 341 行）也换成 `agents_sandbox`（用户 10-19 同意一起换）。
7. 测试：悬停文字三种情况；各主题都有 `agents_sandbox`、和 `agents_accent` 不同。

## 怎么算做完
- 用户原话：“选B，展开卡片和New Agent的锁都统一”；“都同意，一起换，你自己做吧”。
- 主控建议的验收（用户 10-19：“都同意”）：
  1. 三项检查全过（`cargo test --all-targets`、`clippy -D warnings`、`fmt --check`）。
  2. 用户实际看：收起和展开的卡片、New Agent 底边，在深色和浅色主题下胶囊一眼看得出、不刺眼，和主控卡片的蓝色标记分得开；侧栏拖窄时名字那一行不乱。

## 不要做
- 不改 sandbox、fence、corral 的逻辑和读到的数据；不改 Delegates（P5-141）的标记。
- 不起 paddock 窗口、不截图。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-145-sandbox-pill`，分支 `p5-145-sandbox-pill`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-145-sandbox-pill`（从 `.target/main` 克隆）。

## 完成记录（主控 10-19）
### 做了什么
- 主题加 `agents_sandbox`（`preset.rs`：默认 Dune `#6cc4ad`、Tide `#67c9a5`、Lagoon `#8fd18a`、Catppuccin `#94e2d5`、Tokyo Night `#73daca`、Rosé Pine `#9ccfd8`、Kanagawa `#7aa89f`；`[colors]` 里可改，Settings 的颜色页自动多一行）。
- `sidebar_sandbox.rs` 加 `pill`／`pill_width`：淡底（16%）、圆角，实心小锁（11 点）、档案名（半粗），有条数时接黄色 `· 2+`。收起卡片用它（宽度算进名字那一行的余量，强度照旧先让位）；展开卡片的 Sandbox 一格用不带条数的胶囊，这一格的淡底也从强调色换成 `agents_sandbox`。
- 悬停文字：拿满一页时 `Sandbox: dev · N blocked in its latest records`，读全了 `Sandbox: dev · N blocked`，没拦过 `Sandbox: dev`。
- Blocked 浮窗标题的锁、New Agent 底边选了档案的胶囊和档案菜单选中行的锁、Settings › Sandbox 选中档案的锁，从强调色换成 `agents_sandbox`。
- DESIGN §13 P5-120 的卡片一条改写，另加 P5-145 一条。

### 验证了什么
- 先红后绿：悬停文字的新断言改前得 `Sandbox: dev · 5000+ blocked`，改后过。
- 新测试 `a_sandboxed_agents_pill_is_apart_from_the_accent_and_the_count`：每套主题 `agents_sandbox` 和 `agents_accent`、`agents_yellow` 色相至少差 30°（Lagoon 若用青绿会因离强调色太近而不过）；`agents_sandbox` 加进“在侧栏底上读得清”的对比度测试（≥ 4.5:1），七套都过。
- 三项检查：`cargo test --all-targets` 817 过、0 失败；`clippy --all-targets -D warnings`、`fmt --check` 干净。

### 拿主意的地方
- 七套主题都是深色，没有浅色主题；任务文件里“浅色主题看得清”这条无从验，靠对比度测试兜底。
- Lagoon 的强调色本身是青绿，它的 sandbox 色改用浅绿 `#8fd18a`。几套主题的 sandbox 色和 Codex 的种类色相同或很近（都是青绿）；胶囊在名字后面、种类色在头像上，位置分开，没另配。
- `See the rules` 链接、Settings 选中档案卡片的框和底是“链接／选中”的意思，仍用强调色。
- 没有交叉审查：只改画法和一个颜色，不碰逻辑。

### 没做的事
- 没起窗口、没截图：胶囊大小、颜色深浅、侧栏拖窄时的样子留给用户实际看。
