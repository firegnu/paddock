# 任务：看板 Queued 里所有 Needs you 合成一组，不再按理由分

2026-10-26，paddock/main 写。
类型：功能变更
依据：P5-191 按 `待用户：` 一字不差归组，理由写法稍有不同就分成好几组，canter 的看板出现好几个琥珀组夹着单件。用户看了以后要求简单处理：有 `待用户：` 的都归一组，只改画法。
执行：主控

## 用户原话
- 10-26 发来 canter 仓库看板截图（Queued 25 件都写了 `待用户：`，分成 10 件、8 件、2 件三个琥珀组，中间还夹着几件单列的 Needs you）：“现在的needs you太乱了，各种needs you”。
- 主控先提了加一个“未分解”状态的方案，用户：“不要搞这么复杂。”
- 主控改提：Queued 里所有 Needs you 合成一个组，不按理由文字分；默认收着，点开一件一行，理由放悬停；只改画法。用户：“可以，写任务文件吧”。

## 先读
- 仓库根的 `CLAUDE.md`。
- `docs/DESIGN.md` §13 看板的 P5-191 一条（现在的归组规则和画法），以及 P5-29b、P5-113。
- 代码：`app/src/kanban_view.rs` 的 `queued_groups`、`QueueRow`、`queued_rows`、`ask_group`、`opened_asks`，及 `queued_groups` 的单测。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-193-needs-you-one-group`，分支 `p5-193-needs-you-one-group`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-193-needs-you-one-group`（从 `.target/main` 克隆）。只动 `app/src/kanban_view.rs` 和 `docs/DESIGN.md`。

## 要做的
1. **归组条件**：Queued 里（`listed(Queued)` 列出的，草稿也算，Loop 框里的不算）`待用户：` 有内容的任务全部归进同一组，不再比较理由文字。只因 agent 在等而标 Needs you、没有文字的照旧不参加。至少 2 件才成组，只有 1 件时照现在两行的样子画。
2. **位置**：Queued 的排序不变；这一组放在它排得最前的成员的位置，成员在组里保持原顺序，其余任务的先后不变（同 P5-191）。
3. **组头**：样子同 P5-191（淡琥珀底、琥珀细边、折叠箭头、粗体琥珀 `Needs you`、琥珀件数小签），但不再写理由：`Needs you` 后面直接是件数小签。组头不加悬停说明。
4. **收着时**（默认）：组头下面一行淡色小字列成员标题，照旧。
5. **展开时**：成员一件一行，照 P5-191。成员行的标题上加悬停说明，写这件的理由全文（`Needs you: <理由>`）。成员行的悬停按钮和就地确认照旧。
6. **展开状态**：只在这次运行里记，重启回到收着（同 P5-191）。只有一个组了，不再按理由文字记。
7. Queued 整栏收起时（P5-29b）和加宽后的五列，都用同样的归组。
8. 不变：顶部的 `N need you` 计数、Needs you 的来源、Loop、别的栏、DONE。界面文字用英文。
9. `docs/DESIGN.md` 里 P5-191 那一条改成新规则，注明 P5-193 和用户原话。

## 怎么算做完
- 用户原话：Queued 里所有 Needs you 合成一个组，不按理由文字分；默认收着，点开一件一行，理由放悬停；只改画法。
- 验证：
  - 先改 `queued_groups` 的单测，确认因缺这个行为而失败，再实现。至少覆盖：理由文字不同也归进同一组；只有 1 件不成组；没有文字的不参加；组的位置、成员和其余任务的先后不变；草稿参加。
  - `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。
- 画面合并安装后留给用户实际看（canter 的看板就是现成的例子）。

## 不要做
- 不加新的任务状态或标记，不改任务文件格式和约定，不碰 canter 仓库。
- 不改顶部计数、排序和 Loop 的取活规则。
- 不加“全部清掉”按钮，不把展开状态存盘。
