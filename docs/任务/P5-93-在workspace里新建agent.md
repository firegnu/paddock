# 任务：在 workspace 里直接新建 agent

2026-10-14，paddock/main 记。
依据：用户 10-14 发来截图（新建了一个空的 workspace `test`，`⋯` 菜单里只有 `Rename`、`Add repositories…`、`Remove workspace`）：“新建一个workspace之后，看起来没法快速创建一个agent”。主控问入口放哪，用户：“按照你的推荐走”；问开出来的 agent 怎么算进来，用户选“只把这个 agent 记进来”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-93-workspace-new-agent`，分支 `p5-93-workspace-new-agent`（开工时从 main 建）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-93-workspace-new-agent`。
- 动 `app/src/sidebar_workspaces.rs`、`sidebar.rs`（事件）、`window.rs`（打开 New Agent 面板、开起来之后）、`new_agent.rs`（只在需要带上“从哪个 workspace 开的”时）、`workspaces.rs`、相关测试、`docs/DESIGN.md` 和本文件。

## 现在是怎样
New Agent 面板只能从左下角菜单、空状态按钮、菜单 `New Agent…`、⇧⌘N 开，目录预填当前窗格的目录。workspace 里开不了 agent；开出来的 agent 进不进某个 workspace，只看它的主仓库是不是那个 workspace 的成员。空的 workspace 只能先 `Add repositories…` 再去别处开 agent。

## 要做的（入口用户说“按照你的推荐走”，下面三处是主控的推荐）
1. workspace 标题的 `⋯` 菜单最上面加一行 `New Agent…`（和左下角菜单同一个图标）：打开平常的 New Agent 面板，目录预填这个 workspace 的第一个仓库（按名字排）；workspace 里没有仓库时照平常的预填。
2. 空的 workspace（托盘里只有一行 `No agents`）：那一行后面多一个 `New Agent` 按钮，点了同第 1 条。
3. workspace 里整个仓库在、但现在没有 agent 的那一行（`<仓库> No agents`）：鼠标停在这一行时右边出一个 `+` 按钮（提示 `New Agent in <仓库>`），点了打开面板，目录预填这个仓库。
4. 从上面三处开的面板，agent 开起来之后：如果它照现有规则不归这个 workspace（目录不在这个 workspace 的仓库里，比如空 workspace），就按 agent 全名把它单独记进这个 workspace（用户：“只把这个 agent 记进来”）；本来就归这个 workspace 的什么都不记。它所在仓库的别的 agent 不动。判断要读这个目录的主仓库，在后台读，不卡界面。
5. 面板里用户改了目录、名字也照上面算：以开起来时实际的目录为准。从别处（左下角菜单、⇧⌘N 等）开的面板照旧，不记进任何 workspace。

## 怎么算做完
- 用户：“新建一个workspace之后，看起来没法快速创建一个agent”；入口“按照你的推荐走”；归属“只把这个 agent 记进来”。
- 自查：`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过；第 4、5 条的判断（该不该记、记哪个）放在不依赖 GPUI 的地方，先写测试看它失败。

## 不要做
- 不改 New Agent 面板的样子和字段；不在面板里加“加入哪个 workspace”的选项。
- 不改 workspace 的归属规则、存盘格式、别的菜单项。
- 不起测试窗口、不开真的 agent；样子留给用户看。

## 完成记录

2026-10-14，paddock/main 自己做。
- 做了什么：
  - `workspaces.rs`：`Workspaces::join`（从 workspace 开的 agent：照 `of` 已经归这个 workspace 就不记，否则 `add_agent`；workspace 不在了不记；返回记了没有）；`Workspace::first_repo`（按仓库名排第一个）。
  - `new_agent_view.rs`：`Seed` 多 `workspace`、`start_in`；面板记着从哪个 workspace 开的；`new` 时目录用 `start_in`，`reopen` 时有 `start_in` 就换目录（平常照旧保留上次填的）；按 Start 那一刻把 workspace 记下，随 `NewAgentEvent::Started` 带回来。
  - `window.rs`：`open_new_agent` 拆出 `show_new_agent`，新加 `open_new_agent_in`；`Started` 带 workspace 时交给左侧栏 `join_started`；处理 `SidebarEvent::NewAgentIn`。
  - `sidebar.rs`：`SidebarEvent::NewAgentIn { workspace, start_in }`；`vacant` 的调用带上 workspace 名。
  - `sidebar_workspaces.rs`：`new_agent_in`（关掉浮着的菜单、重读名单、算预填目录、发事件）；`join_started`（后台 `activity::main_repository` 后 `edit_workspaces` 里 `join`）；`⋯` 菜单最上面 `New Agent…`；`vacant` 加按钮：空 workspace 一直显示 `New Agent`，仓库那行悬停才显示 `+`，按钮高度不撑高那一行。
  - `docs/DESIGN.md` 加 P5-93 一条。
- 验证了什么：
  - 先写 `tests/workspaces.rs` 的 `an_agent_started_from_a_workspace_is_recorded_there_unless_its_repository_brings_it`，`join`、`first_repo` 各先放一个什么都不做的空实现，跑过看到断言失败，再实现后通过。覆盖：仓库已在 workspace 里不记；空 workspace、Git 之外的目录只记 agent；仓库在别的 workspace 里时只把 agent 记过来、仓库留原处；在别处单独记着的名字挪过来；workspace 不在了不记；预填目录按仓库名取第一个、没有仓库是 None。
  - `app/` 下带本任务的编译目录：`cargo test --all-targets` 全过（各测试程序合计 581 个）、`cargo clippy --all-targets -- -D warnings` 退出码 0、`cargo fmt --check`、`git diff --check` 通过。
  - 没起窗口、没开真的 agent；面板、按钮、`join_started` 那一段界面和异步代码没有自动测试，留给用户实际用。
- 拿主意的地方：
  - `join_started` 每次都写一次 `workspaces.json`（不用记时内容不变），省掉“先判断再写”的两遍。
  - agent 开起来到记进 workspace 之间隔着一次 Git 查询（很快），这期间它可能先出现在 workspace 外面，记好后挪进来。
  - 空 workspace 的按钮一直显示（那一行本来就是空的，用户要的是快）；仓库那行的 `+` 只在悬停时出现，和仓库小标题的按钮一样。
- 没做的事：面板里选加入哪个 workspace；快捷键。
