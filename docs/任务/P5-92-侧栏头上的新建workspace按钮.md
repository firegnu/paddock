# 任务：左侧栏标题栏那一行加一个常驻的新建 workspace 按钮

2026-10-14，paddock/main 记。
依据：用户 10-14 用过 P5-89 后一开始找不到新建 workspace 的入口；主控提议在侧栏头上加一个常驻的入口，用户：“我觉得要加”；主控给了三个位置（标题栏那一行、列表最上面一行、左下角按钮旁边），用户选了“标题栏那一行”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-92-workspace-button`，分支 `p5-92-workspace-button`（开工时从 main 建）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-92-workspace-button`。
- 只动 `app/src/sidebar.rs`（标题栏那一行、窄条、最窄宽度）、`app/src/window.rs`（左下角菜单）、图标所在的地方、相关测试、`docs/DESIGN.md` 和本文件。

## 现在是怎样
新建 workspace 只有三个入口：菜单 View ▸ `New Workspace…`、命令面板、仓库小标题悬停才出的按钮菜单。左侧栏标题栏那一行是：收起侧栏的按钮、`Agents` 和数字，右端是暂停全部的按钮（有活着的 agent 时才有）和铃铛。左下角 `⋯` 菜单里有 `New Agent…`、`New Shell`、`Sort`、`Fold Agents`、`Settings…`，没有新建 workspace。

## 要做的
1. 标题栏那一行，在暂停全部的按钮前面（没有暂停键时在铃铛前面）加一个按钮：图标用 P5-89 已有的“层叠加号”（`sidebar_workspaces.rs` 的 `LAYERS_PLUS`），大小、颜色、悬停底色和暂停全部的按钮一样，悬停提示 `New Workspace…`；点了和 View ▸ `New Workspace…` 一样打开新建浮层。一直在，没有 agent、没有 workspace 时也在。
2. 侧栏最窄能拖到多宽、铃铛什么时候变成紧凑的，都把这个按钮的宽度算进去，标题栏那一行在最窄时不挤、不重叠。
3. 侧栏收成窄条时，这个按钮放在暂停全部的按钮下面（没有暂停键时在铃铛下面），悬停提示一样。
4. 左下角 `⋯` 菜单在 `New Shell` 下面加一行 `New Workspace…`，同一个图标，点了打开新建浮层。

## 怎么算做完
- 用户：“我觉得要加”；选“标题栏那一行”。
- 自查：`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过；最窄宽度的测试改成算上这个按钮并先看它失败；`⋯` 菜单项的测试（如有）跟着改。

## 不要做
- 不改新建浮层本身、workspace 的规则、菜单 View 里那一项、仓库小标题上的按钮。
- 不加快捷键。
- 不起测试窗口；样子留给用户看。
