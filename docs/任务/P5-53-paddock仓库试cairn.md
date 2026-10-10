# 任务：paddock 采用 cairn，和现在的 HANDOFF 并存试一两周

2026-10-10，paddock/main 写，主控自己做（一条命令加 HANDOFF 一行）。
类型：其他
依据：
- 用户 10-08：“cairn迁移到paddock中开始测试cairn”。10-09 看过 owlet 的 `cairn show`：“这个内容我觉得够了”。
- 用户 10-10：“我想开始做P5-55，但是我还是想知道现在的handoff信息会比cairn记录的会多多少？”主控对着 owlet 的记录和 cairn 的设计文档比：HANDOFF 约三分之一是进度（cairn 会记），其余是环境和做法、留给用户顺带看的清单、悬着的事和主控做法备忘（cairn 的记录格式里没有位置，注入总量也只有 6,000 字符）。
- 用户：“那么意思就是说，cairn理论上就是将三分之一的摘过去，但是handoff还要保留？”主控建议先 adopt、两边并存一两周，cairn 接续得准再动 HANDOFF。用户：“我觉得可以。你的意思是说，保留现在全量的handoff，每次做完我还是要提醒handoff，但是同时cairn同时作用。是这个意思吗？”主控说明 HANDOFF 是照 `AGENTS.md` 的规矩在收尾时更新、不用提醒，cairn 是钩子自动存。用户：“我知道了，那么按照现在的逻辑，我指甲上cairn的话，就是会多一个cairn的记录，对我完全没有影响。之后运行一段事件我还能对照”。
执行：主控

## 在哪里干活
- 不建 worktree：不改代码。`cairn adopt` 记在 cairn 自己的库里（按仓库的 git 公共目录认项目，所有 worktree 一起生效），不进仓库；仓库里只改 `HANDOFF.md` 一行。

## 要做的
1. 在 paddock 仓库跑 `cairn adopt`。
2. HANDOFF 全量保留：主控照旧在每件活收尾后自己更新（不用用户提醒），开新会话照旧先读 HANDOFF。规矩（`AGENTS.md`）不改。
3. cairn 同时起作用：主控每个回合结束前存一次停点（cairn 的约定，自动的）；开新会话时 cairn 把最近的记录注入进来。派出去的 agent 照旧带 `CAIRN_DISABLE=1`，不用 cairn。
4. HANDOFF 里“cairn 已全局装进 Claude Code……paddock 还没有”那一条改成已采用、哪天开始试、怎么退回。
5. 试点看什么：新会话里 cairn 注入的停点、下一步、待用户决定准不准，有没有漏记、错记；和 HANDOFF 对不上时以哪边为准。一两周后（10-24 前后）和用户定：HANDOFF 里“进度”那三分之一删不删，其余长期内容要不要拆去别处。

## 不做
- 不删、不瘦 HANDOFF；不改“先读 HANDOFF”的规矩。
- 不改 cairn；cairn 面板是 P5-55。

## 怎么算做完
- 用户原话：“cairn迁移到paddock中开始测试cairn”“保留现在全量的handoff……但是同时cairn同时作用”。
- `cairn status` 在 paddock 里显示项目已采用；HANDOFF 那一条已改并提交。
- 退回办法：`cairn unadopt`，记录留在 cairn 的库里不删。

## 完成记录

- 做了什么：在 paddock 仓库跑了 `cairn adopt`（输出 `adopted …/paddock/.git`）。HANDOFF 里 cairn 那一条改成已采用、10-10 起试、10-24 前后回看、怎么退回；“下一步”第 1 条改成“P5-53 试点中”。规矩（`AGENTS.md`）没改，HANDOFF 没删内容。
- 验证了什么：`cairn status` 在 paddock 里显示“项目：…/paddock/.git（已采用）”。
- 拿主意的地方：用户说的是对并存方案的理解（“多一个cairn的记录，对我完全没有影响。之后运行一段事件我还能对照”），前面说过“我觉得可以”，主控据此直接 adopt；这一步随时能用 `cairn unadopt` 退回。
- 没做的事：这个会话是采用之前开的，没有 cairn 的注入；注入准不准要从下一个新会话起看。试点结论（HANDOFF 瘦不瘦、“先读 HANDOFF”改不改）留到 10-24 前后和用户定。
