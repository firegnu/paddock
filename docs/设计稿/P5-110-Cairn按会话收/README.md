# P5-110 Cairn 记录按会话收样稿

用户 10-15 发来 Cairn 标签截图：“现在的太长了……一眼望去密密麻麻。”主控查到主要是同一个会话 0.4.0 之前每个回合 `cairn save` 的 26 条旧记录一条条列着；提议按 `cairn list --json` 的 `source_id`（cairn DESIGN §7.1 公开约定）一个会话一行，会接续的（`show --json` 的 `record_ids`）照旧单独突出、标题只写一次。用户：“可以，出样稿吧”。

`sessions.html` 一个文件里并排三屏，直接用浏览器打开。配色取 lagoon 主题的近似值，实现时一律从主题取色；记录内容、时间、编号是照截图改写的示意；Claude／Codex 的小图标是占位，实现时用现有的 agent 图标。

| 屏 | 内容 |
| --- | --- |
| 甲 一个框 | 会接续的几条合进一个强调色框，`Next session starts from these` 只写一次，条间细线；有更早记录的标 `26 earlier`；下面 `Earlier` 按天分组、每个会话一行、一行摘要；分支和顶上一样不画，不一样才画（示意里 `p5-94` 那条）；哪家用小图标 |
| 乙 卡片加左线、Earlier 收起 | 会接续的仍是分开的卡片，不重复标题，左边一道强调色线；`Earlier · 5 sessions` 默认收成一行，点开才列 |
| 甲 点开 | 点开 14:49：全文和 `Saved／By／Branch／Record` 照 P5-79；最下面 `Earlier in this session · 26`，默认露 3 条加 `Show all 26`，点一条再看它的全文 |
