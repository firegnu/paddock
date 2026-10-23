# 任务：用 corral、ranch dispatch、cairn 的仓库，从 dispatch 到 cairn 到 AGENTS.md 和 HANDOFF 整理统一

草稿（2026-10-15，paddock/main 记）。只记了用户的话，范围和验收还没定，开工前和用户过一遍。
依据：用户 10-15：“加一个draft task，要收拾一下所有用corral，ranch dispatch，还有cairn的所有repo要从dispatch到cairn再到agents.md以及handoff都要整理统一一下”。

## 现状（主控 10-15 粗查，只看了文件在不在、提没提到，开工前再核对）
- `~/Developer/personal_projs/` 下 AGENTS.md、CLAUDE.md 或 HANDOFF.md 里提到 corral、dispatch 或 cairn 的仓库：cairn、corral、dispatch-log、drover、global-mesh、herdr-interactive-worker、herdr-loop-lab、jb-finetune、owlet、paddock、pasture、ranch、saddle、saddle-e2e-20261002-113520、task_distributed_test。哪些是真在用、哪些只是旧提及，还没分。
- `ranch dispatch` 有 `route` 和 `install-skills`（装到 claude、codex、gemini），各仓库的派活说法目前各写各的。
- cairn 0.4.x 起接续摘要由 cairn 写，paddock 的 AGENTS.md 和 HANDOFF.md 已按它改过（HANDOFF 只记项目级的账）；别的仓库还没对过。

## 要先和用户定的（主控提议）
1. 范围：上面哪些仓库算“在用”；saddle 按 paddock 的规矩不能改，它和 saddle-e2e 是只列出差异交给用户／Saddle 主控，还是这次用户另外授权。
2. 统一成什么：一份共用的写法（派活走 `ranch dispatch`、会话接续交给 cairn、AGENTS.md 写规矩、HANDOFF 只记项目级的账）放在哪里当底本，各仓库照抄还是引用。
3. 怎么改：每个仓库有自己的规矩和主控，是 paddock 主控逐个去改，还是写成需求发给各仓库的主控（像 cairn/main 那样）在各自的流程里做。
4. 验收：用户原话之外，用什么算“统一了”。

## 不做（主控提议）
- 不改 corral、ranch、cairn 的代码；公开约定要变的写成需求交给对应仓库。

## 完成记录

2026-10-23 收尾，paddock/main 记。
- 范围：用户 10-23 按 paddock、ranch、cairn 三个仓库算（“我记得三个都整理了”）。上面列的其他仓库这次不动，以后要整理另开一件。
- 统一成：HANDOFF 只记照着执行的事实（装的哪一版、怎么退回、机器和环境的长期事实、还没派的草稿、已知没修的问题），会话经过、进度、待定的事、约定和经验由 cairn 记；派活走 `ranch dispatch`。以 ranch 10-22 的写法为底本，各仓库照抄进自己的 AGENTS.md。
- 怎么改的：每个仓库在自己的流程里改。paddock 已改（AGENTS.md「先读」和「开发方式」两处）；ranch 自己改（ranch `f1f8cc9`）；cairn 由 paddock/main 写成需求发给 cairn/main（用户：“发给 cairn/main 吧，让他改去。”），cairn/main 改了 AGENTS.md 两处、重写 HANDOFF.md（cairn `2fcc007`），cairn 的 CLAUDE.md 是指向 AGENTS.md 的链接。
- 验证：主控只读核对了三个仓库 AGENTS.md 里 HANDOFF 的两处写法一致。
- 没做：其他仓库；corral、ranch、cairn 的代码没动。
