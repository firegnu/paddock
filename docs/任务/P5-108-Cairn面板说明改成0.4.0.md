# 任务：Cairn 面板的说明文字改成 cairn 0.4.0 的说法

依据：用户 10-15 经 cairn/main 转达：“paddock 右侧栏 Cairn 面板，说明文字还是 cairn 0.3.0 的说法，请改成 0.4.0 的。只改文字，不用改别的。”0.4.0：记录是 cairn 在后台读会话记录、调旁路模型写的摘要（会话安静 3 分钟、会话结束、或同一目录新会话开局时写），agent 不用保存；新会话开局给最近 3 个会话的全文、更早的各一行。“Next session starts from this” 的标记不动。
执行：主控

## 在哪里干活
- worktree：`../paddock-worktrees/p5-108-cairn-copy`，分支 `p5-108-cairn-copy`（已从 main 建好）。

## 要改的文字（`app/src/cairn_view.rs`）
1. 记录列表顶上的说明（转达点名的这处）：
   - 现在：`Agents write one when a turn ends. A new session in this repository starts from the newest.`
   - 改成：`cairn summarizes each session in the background once it has been quiet for 3 minutes or has ended. A new session in this repository starts from the most recent ones.`
2. 没采用的仓库，`Adopt…` 上面那句（同样是 0.3.0 的说法，主控查到的）：
   - 现在：`Once adopted, the agents you talk to here save a short handover note as each turn ends, and the next session starts from it.`
   - 改成：`Once adopted, cairn summarizes the sessions you have here in the background, and each new session starts from the most recent ones.`
3. Adopt 确认卡片的第一段（同上）：
   - 现在：`This takes effect right away. Claude and Codex sessions you talk to in this repository, the open ones included, are asked to save a handover note before each turn ends, starting with their next turn.`
   - 改成：`This takes effect right away. cairn summarizes the Claude and Codex sessions you talk to in this repository in the background, the open ones included from their next turn, and new sessions start from those summaries.`
   - 第二段 `Dispatched agents are not affected. To undo, run \`cairn unadopt\` in this repository.` 不动（派出去的 agent 照旧带 `CAIRN_DISABLE=1`）。

## 和 cairn/main 对过（10-15，用户：“你直接和cairn的主控交流。达成统一之后，你直接改”）
- 第 1 句：同目录新会话开局也写是兜底，面板不提；quiet 写具体成 3 分钟。
- 第 3 句：采用后开着的会话在下一个回合结束时被 hook 记下、起后台写摘要，所以加回 the open ones included。
- 第 2 句、第二段、标题、标记都同意不动。

## 怎么算做完
- 用户原话：“说明文字还是 cairn 0.3.0 的说法，请改成 0.4.0 的。只改文字，不用改别的。”

## 不做
- 不动标题 `Handover notes`、`→ Next session starts from this` 标记和面板的读取、画法。
- DESIGN §13 P5-55／P5-79 没写这几句的原文，不用改。

## 完成记录（2026-10-15，paddock/main）
- 做了什么：`app/src/cairn_view.rs` 三处说明文字照上面和 cairn/main 对过的英文改了，别的没动。
- 验证：纯文字改动，没有测试断言这几句；`cargo test --all-targets` 全过（合计 660 项），`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 干净；改后的字符串行都在 100 列以内。主控没起窗口看过，换行效果留给用户看（第 1 句比原来长一行）。
- 拿主意的地方：第 2、3 处不在转达里，是主控查到、和 cairn/main 对过后一起改的（用户：“达成统一之后，你直接改”）。
- 没做的：标题、`→ Next session starts from this` 标记、面板读取和画法都没动。
