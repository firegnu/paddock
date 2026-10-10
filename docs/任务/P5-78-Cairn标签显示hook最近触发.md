# 任务：Cairn 标签的状态行显示每家 hook 最近一次真正触发的时间

2026-10-10，paddock/main（主控自己做）。用户 10-10 看过任务文件：“P5-78都按你的建议来，开始做吧”。
路由：没问路由（主控自己做）；影响面：改行为。
类型：功能变更
依据：用户 10-10 做 P5-55 时说要“来确定cairn工作正常”。P5-55 的面板只知道 hook 装没装，分不出“装了但没触发”（比如 Codex 的 hook 还没在 `/hooks` 里信任，面板照样打勾）。主控提议请 cairn 在 `cairn status --json` 里报每家 hook 最近一次触发的时间，用户：“那两项都提吧， 你直接派活给cairn的主控，让他去做。你的活你自己做”。cairn 那一半后来用户改成让 paddock/main 直接在 cairn 仓库做（“我取消了，我觉得你直接再cairn中干吧…另外我觉得问题不要最小化操作。就是一次解决。”），见 cairn 仓库 `docs/tasks/F2-hook最近触发与公开约定.md`。
依赖：P5-55
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-78-cairn-seen`，分支 `p5-78-cairn-seen`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-78-cairn-seen`。

## cairn 给的东西（cairn 0.2.0，F2）
`cairn status --json` 里每家多一个 `last_seen`，是当前仓库里这家 agent 四种 hook 各自最近一次被 cairn 处理的时间，没有记录是 `null`：

```json
"agents": {
  "claude": { "installed": true,
              "last_seen": { "SessionStart": "2026-10-10T08:50:43.687Z", "UserPromptSubmit": "2026-10-10T09:41:02.118Z",
                             "Stop": "2026-10-10T09:43:55.004Z", "SessionEnd": null } },
  "codex":  { "installed": true,
              "last_seen": { "SessionStart": null, "UserPromptSubmit": null, "Stop": null, "SessionEnd": null } }
}
```

cairn 那边写明的三个限制（cairn DESIGN §8.7），界面的措辞不能超出它们：
- 没采用的仓库不记，全是 `null`；采用之前触发过的不算。
- 带 `CAIRN_DISABLE=1` 的会话不记（派出去的 agent 都带），所以“只有派出去的 agent 在这个仓库里干过活”也是全 `null`。
- hook 触发了但处理出错不记。所以“没有时间”可能是没触发，也可能是每次都出错。
- 0.1.0 的 cairn 没有这个字段。

## 要做的
- `app/src/cairn.rs`：读 `last_seen`，每家取四个时间里最晚的一个。没有这个字段（旧版 cairn）时当作“不知道”，状态行照 P5-55 现在的样子画，不算出错。
- `app/src/cairn_view.rs` 状态行（只在已采用的仓库里加东西；没采用的仓库照旧，因为那里一定全是 `null`）：
  - 装了、有时间：勾后面写离现在多久，淡色，写法和卡片上的时长一样（`45s`、`2m`、`3h`、`2d`）。
  - 装了、四个都是 `null`：勾换成黄色的 `never`。
  - 没装：照旧一道短横。

  ```
  平时            Hooks   Claude ✓ 2m   Codex ✓ 3h                0 uncollected
  Codex 没触发过   Hooks   Claude ✓ 2m   Codex never（黄）          0 uncollected
  旧版 cairn      Hooks   Claude ✓      Codex ✓                   0 uncollected
  ```
- 时间跟着面板每 5 秒一次的读取更新，不另起定时器。
- `docs/DESIGN.md` §13 加一条，§3 的 cairn 一行补上 `last_seen`，并写明这些输出现在是 cairn DESIGN §7.1 的公开约定。
- 测试只用假 `cairn` 脚本：有时间、全 `null`、没有这个字段、没采用四种。

## 用户定了的（10-10：“P5-78都按你的建议来，开始做吧”）
1. 状态行只写每家“最近一次”，不把四种事件分开列。
2. `never` 下面不加解释的小字；不放心时自己跑 `cairn status` 看。

## 怎么算做完
- 用户没给验收原话；上面“要做的”逐条达到。
- `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过。
- 样子留给用户看（主控不起窗口）。

## 完成记录

2026-10-10，paddock/main。

- **做了什么**：`app/src/cairn.rs` 读每家的 `last_seen`，取最晚的一个，在读的时候算成 `Fired`（`Unknown`／`Never`／`Ago("2m")`），`Found` 多两个字段 `claude_fired`、`codex_fired`；`read` 多一个参数“现在几点”（测试用固定时间）。`app/src/cairn_view.rs` 状态行：有时间在勾后面淡色写多久，装了从没触发过把勾换成黄色 `never`，其余照旧。DESIGN §13 加一条、§3 的 cairn 一行改成公开约定的说法。
- **验证**：先写测试并看到它因为没实现而失败（期望 `Ago("2m")`、`Never`，得到 `Unknown`），再实现。`app/tests/cairn.rs` 新增一项，用假 `cairn` 脚本：取四个里最晚的；秒、小时、天的写法；时间比本机钟还晚算 `0s`；没采用（`not_adopted`、`no_data`）和没装 hook 的那家是 `Unknown`；没有这个字段（旧版）是 `Unknown`；读不出来的时间是 `Unknown`。自查时发现年份离谱的时间会让算天数溢出（调试版会崩），补了用例（先看到它崩）并把年份限制在 1970 到 9999。`cargo test --all-targets` 542 项通过，`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 干净。
- **拿主意的地方**：
  - 时长在读的时候算成文字存进 `Found`，不存时间戳：面板只在读到的东西变了才重画，存时间戳的话 agent 闲着时画面上的“2m”会一直不动。
  - 时间自己解析（cairn 固定写 `2026-10-10T09:43:55.004Z` 这种），不为此加日期库；四个时间里有一个读不出来就整家当作不知道，不猜。
  - 小时的写法跟卡片一样是 `3.0h`（任务文件的示意里写的是 `3h`）。
  - 没装 hook 的那家即使 cairn 里留着旧时间也不画（已经卸了）。
- **没做的**：没起窗口看样子（留给用户）；界面字号调大后这一行会不会挤没验证（各项都不压缩，放不下时右边的 `N uncollected` 先被裁）；没把四种事件分开列、`never` 下面没加解释（用户定）。
