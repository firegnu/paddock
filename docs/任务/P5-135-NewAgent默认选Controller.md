# 任务：New Agent 打开时默认选 Controller 预设

依据：用户 10-17（附截图，New Agent 打开时亮的是 Developer）：“默认改为controller，现在是developer”。主控查明：New Agent 打开时写死成“普通 Claude、默认模型、normal 强度”，正好对上 Developer；Controller 的名字固定是 `<前缀>/main`，表单不查重名，项目已有 main 时要到 corral 启动才报错。问用户后选了“有 main 就退回 Developer”（只看打开那一刻，换项目不跟着变）。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-135-default-controller`，分支 `p5-135-default-controller`（已从 main 建好）。

## 要做的
- `app/src/new_agent.rs` 的 `Form` 加打开时的设置：`<前缀>/main` 还没有时，套用预设里第一个 Controller 角色的；已有 main、或预设里没有 Controller 角色的，照旧（普通 Claude、默认模型、normal 强度）。
- `app/src/new_agent_view.rs` 打开时改用它。
- `docs/DESIGN.md` P5-37 一条补一句打开时默认亮哪个。

## 怎么算做完
- 用户原话：“默认改为controller，现在是developer”；用户选的：“有 main 就退回 Developer”。
- 先在 `new_agent.rs` 写测试（没有 main 时亮 Controller、已有 main 时亮 Developer），确认红，再改到绿；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check`。
