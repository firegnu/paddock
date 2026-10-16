# 任务：Cairn 面板单独显示 cairn 0.6.0 的「经验」

2026-10-16，paddock/main 自己做。
类型：功能变更
依据：cairn/main 10-16 转达用户（用户让 paddock 做界面）：cairn 0.6.0 已装到真实环境，加了经验沉淀；`cairn ledger --json` 的条目 kind 多了 `lesson`（经验），字段和账的三类一样；“经验”和会话记忆、项目的账并列，面板上要单独一组或一个标签显示，不要混在账的三类里；不认识的 kind 不能出错，按普通条目显示；`cairn show --json` 的 `ledger_item_ids` 也包括经验，排在账后面。约定以 cairn 仓库 `docs/DESIGN.md` §7.1、D15 为准。用户 10-16 选了「甲：第三个胶囊」；看效果只用假数据，不往真实的账里加条目。
执行：主控
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。

主控自己做的理由：只动 `cairn.rs` 的分类和 `cairn_view.rs` 的切换，P5-118、P5-118a 是主控写的，上下文在手上；只读显示，影响面小。用户 10-16 同意。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-121-lessons`，分支 `p5-121-lessons`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-121-lessons`（从 `.target/main` 克隆）。
- 动 `app/src/cairn.rs`、`app/src/cairn_view.rs`、`app/tests/cairn.rs`、`docs/DESIGN.md` 和本文件。

## 要做的（照用户选的甲）
1. Cairn 标签的切换从 `Notes | Ledger` 变成 `Notes | Ledger | Lessons`，沿用 P5-118a 的靠左小胶囊；`Lessons` 后面带经验条数（没有就不带）。
2. `Lessons` 页只显示 kind 为 `lesson` 的条目，画法和 Ledger 页的条目一样（编号、标题、点开看正文和来源，下次会话收不收到）；右边一句 `Learned across sessions by cairn`。
3. Ledger 页照旧只显示账的三类（State、Open、Conventions），胶囊上的条数不再算经验。
4. 不认识的 kind 照旧进 `Other`，显示在 Ledger 页最后，不出错。
5. 没有经验时 `Lessons` 页写明还没有，以及怎么来（旁路模型记下，或 `cairn ledger add lesson`）；账读不出来时两页都说原因。
6. 只读，不加写操作；不加 `--all`。

## 怎么算做完
- 用户原话（经 cairn/main）：“经验”和会话记忆、项目的账并列，面板上要单独有一组或一个标签显示经验，不要混在账的三类里；遇到不认识的 kind 不能出错，按普通条目显示。
- 先加一条测试：假 `cairn` 给出账、经验、不认识的类，经验单独成一组排在账的三类之后、`Other` 之前，`ledger_item_ids` 里的经验标成下次会话收到。改之前它应当失败（经验现在进了 `Other`）。
- `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check`。
- 不往真实的账里加条目；界面的样子留给用户实际看（等旁路模型记下第一条经验）。

## 不要做
- 不调 `cairn ledger add/edit/close/delete` 等写命令；不读 cairn 的内部状态。
- 不动 Notes 页和 Ledger 页条目的画法。
