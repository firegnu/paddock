# 任务：New Agent 内置的 Controller 预设，强度从 xhigh 改成 high

依据：用户 10-15：“现在new agent的时候，预设的controller的claude的effort是xhigh。我要变成high。”
执行：主控

## 在哪里干活
- worktree：`../paddock-worktrees/p5-107-controller-high`，分支 `p5-107-controller-high`（已从 main 建好）。

## 现状
- 内置的四个预设在 `app/src/new_agent.rs` 的 `default_presets()`，Controller 是 Claude、`opus[1m]`、`xhigh`、主控。配置文件里没写 `agent_presets` 时用这一份（用户的配置没写）。
- DESIGN §13 P5-37 那条写着默认 Controller 是 `opus[1m] xhigh`。

## 要做的
1. DESIGN §13 P5-37 那条的默认 Controller 改成 `high`，注明 P5-107 和用户原话。
2. `default_presets()` 里 Controller 的强度改成 `high`；测试里断言 Controller 预设是 xhigh 的地方跟着改。
3. 重新打包安装。

## 怎么算做完
- 用户原话：“预设的controller的claude的effort是xhigh。我要变成high。”

## 不做
- 不动其他三个预设，不动已写进配置文件的预设（用户自己存过的照旧）。
- 不动已经开着的 agent。

## 完成记录（2026-10-15，paddock/main）
- 做了什么：`default_presets()` 里 Controller 的强度 `xhigh` 改成 `high`；DESIGN §13 P5-37 那条的默认 Controller 跟着改，注明 P5-107 和用户原话。
- 验证：先把 `a_preset_fills_the_kind_model_effort_and_role_only` 的断言改成 `high`，跑过确认失败（预设填的还是 `xhigh`），改代码后通过；`cargo test --all-targets` 全过（库内 534 项及各测试程序），`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 干净。
- 拿主意的地方：没有。
- 没做的：其他三个预设、已写进配置文件的预设、开着的 agent 都没动。用户的配置没写 `agent_presets`，装上后 New Agent 点 Controller 就是 `high`。
