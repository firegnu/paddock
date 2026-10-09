# 任务：主控自己做的活，Kanban 卡片上显示主控在干活

2026-10-09，paddock/main 写，主控自己做。
类型：功能
依据：
- 用户 10-09：“再加一个draft任务，现在主控干活的时候，kanban看不到是主控在干活。”（P5-58、P5-66 都是主控自己做的，进行中时卡片写 “No agent”。）
- 用户 10-09 选：认法“A 任务文件写一行”，显示“头像加实时状态”。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-67-kanban-controller`，分支 `p5-67-kanban-controller`（已从 main 建好）。

## 现状
- Kanban 认 agent 只有两种办法（`kanban.rs` 的 `agent_for`）：agent 带 `--label task=<编号>`，或工作目录就是任务的 worktree。主控开在主仓库目录、没有 `task` 标签，自己做活时两种都对不上。
- 所以卡片因为 worktree 在而进 In progress，状态却是灰色的 “No agent”。主控只在 To review 列作为审查的人显示（`controller_for`）。

## 要做的
1. 任务文件开头（「依据」之后，和 `依赖：`、`待用户：` 一样只认开头、不认后面引用的）可以有一行 `执行：主控`，`kanban.rs` 读进 `Task`。
2. 这样的任务在 In progress 列、又没有派出去的 agent 时，卡片上显示这个仓库的主控（`controller_for` 找到的那个）：头像、名字、实时状态，和派出去的 agent 一样；主控在等用户时同样标 Needs you。
3. 只影响显示：卡片在哪一列照旧由 worktree、完成记录、合并和收尾决定，主控的回复以 DONE 结尾也不会把卡片推进 To review；任务文件写好、worktree 还没建时仍在 Queued。
4. AGENTS.md 看板约定加一句：主控自己做的活在「依据」后加 `执行：主控`。DESIGN §13 加 P5-67 条。

## 不做
- 不改 corral／ranch（看板约定只在 paddock 的 AGENTS.md，ranch 的派活技能里没有这些行，不用跟着改）。
- 带 `执行：主控` 却有带 `task` 标签的 agent 时，以那个 agent 为准，不另显示主控。

## 怎么算做完
- 用户原话：“现在主控干活的时候，kanban看不到是主控在干活。”选“A 任务文件写一行”“头像加实时状态”。
- `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。
- 本任务自己的卡片在做的过程中就应显示主控，留给用户在看板上看。

## 完成记录

- 做了什么：`kanban.rs` 的 `Task` 加 `by_controller`，任务文件开头（还没进任何一节时）的 `执行：主控`／`执行: 主控` 把它设为真；`board()` 里这件活在 In progress、`agent_for` 没找到 agent 时，用 `controller_for` 找到的主控当作卡片的 agent（状态文字、头像、Needs you 都按它），分列、卡片时间仍只看任务自己的 agent。`kanban_view.rs` 的悬停按钮：卡片的 agent 是主控时只给 Go to agent，不给 Show changes，按钮条宽度跟着少算一个按钮。AGENTS.md 看板约定加 `执行：主控` 一条；DESIGN §13 加 P5-67 条。本任务文件自己开头就有这一行。
- 验证了什么：先写两项测试、看到因缺功能失败（解析恒为假；卡片上没有主控），再实现。解析：开头的两种写法认；完成记录里引用的、`执行：dev`、没有这一行的都不认。看板：没建 worktree 的仍在 Queued 且没人；进行中的显示主控、状态是它的 Working，主控回复以 DONE 结尾也不进 To review；有带标签的 dev agent 时显示 dev agent；没写这一行的照旧 “No agent”；主控 Waiting 时卡片 Needs you。`app/` 下 `cargo test --all-targets`（456 项全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都过。
- 拿主意的地方：Show changes 对主控不给——Changes 标签显示的是 agent 自己工作目录的改动，主控的是主仓库，点了会看到 main 的改动而不是这件活的，容易误会；要看这件活的改动，以后可以另做“按 worktree 显示”的入口（没做）。
- 没做的事：真看板上的样子留给用户：下一件主控自己做的活进行中时，卡片应显示主控（本任务合并后已进 Merged／Done，看不到进行中的样子）。
