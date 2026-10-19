# 任务：卡片上的 sandbox 锁更明显

依据：用户 10-19 截了左侧栏的图（`paddock/dev-terminal-core` 那张卡，箭头指着强度 `high` 后面的小锁），说：“再加一个任务，这个锁能不能明显一点？”主控出了样稿（sandbox 画布 F 块，`docs/设计稿/P5-120-sandbox界面/Lock.dc.html`：现在、A 大锁、B 小胶囊、C 压在图标角上），用户：“选B，展开卡片和New Agent的锁都统一”。
执行：主控
（理由：只改界面上画锁的三处和一个主题颜色，不碰 sandbox、fence、corral 的逻辑；样稿是主控画的，代码位置已核对；量小，派出去的来回比自己做还长。主控不起窗口，样子留给用户实际看。）

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
6. 测试：悬停文字三种情况；各主题都有 `agents_sandbox`、和 `agents_accent` 不同。

## 怎么算做完
- 用户原话：“选B，展开卡片和New Agent的锁都统一”。
- 主控建议、待用户确认的：
  1. 三项检查全过（`cargo test --all-targets`、`clippy -D warnings`、`fmt --check`）。
  2. 用户实际看：收起和展开的卡片、New Agent 底边，在深色和浅色主题下胶囊一眼看得出、不刺眼，和主控卡片的蓝色标记分得开；侧栏拖窄时名字那一行不乱。
- 主控建议一起换、待用户定：Denials 弹框标题里的锁（`sidebar_sandbox.rs` 约 274 行，现在蓝色）和 Settings › Sandbox 档案列表里的锁（`settings_sandbox.rs` 约 341 行）也说的是“这个 agent 自己的墙”，要不要也换成青绿色。

## 不要做
- 不改 sandbox、fence、corral 的逻辑和读到的数据；不改 Delegates（P5-141）的标记。
- 不起 paddock 窗口、不截图。
