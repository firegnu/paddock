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

## 完成记录

- 做了什么：`new_agent.rs` 的 `Form` 加 `open(presets)`：先照旧设成普通 Claude（默认模型、normal 强度），`<前缀>/main` 还没有时再套用预设里第一个 Controller 角色的；`new_agent_view.rs` 打开时改用它。DESIGN P5-37 一条补了一句。
- 验证：先写 `it_opens_as_the_controller_preset_unless_the_project_has_a_main`（没有 main 亮 Controller；已有 main 亮 Developer、名字 claude-1；预设里没有 Controller 角色时照旧），只放旧行为时跑出红（`!form.regular` 不成立），补上后绿；`app/` 下 `cargo test --all-targets` 全过（lib 595），clippy、fmt、`git diff --check` 干净。没起窗口，样子留给用户看。
- 拿主意的地方：按角色找（第一个 Controller 角色的预设），不按名字 “Controller”，用户自己改了预设名也认；用 `apply` 不用 `pick`，和以前一样靠“表单设成哪个就亮哪个”，不算点过。
- 没做的事：换项目、侧栏 agent 名单变了都不重新判断（用户选的“只看打开那一刻”）；表单不在提交前查 `main` 重名。
- 踩过的坑：没有。
