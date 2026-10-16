# P5-118 Cairn 标签显示项目的账样稿

cairn 0.5.0 加了“项目的账”（cairn DESIGN §7.1：`cairn ledger --json`、`show --json` 的 `ledger_item_ids`），cairn/main 10-16 转达请 paddock 面板展示。用户定：面板只读，不做写的命令；放哪里“先出样稿再定”。

`ledger.html` 一个文件里并排四屏，直接用浏览器打开。配色取 lagoon 主题的近似值，实现时一律从主题取色；凸起的框照 P5-117；条目内容取自本仓库 10-16 写进账的条目（节选）。

| 屏 | 内容 |
| --- | --- |
| 甲 标签里切两页 | 状态行下 `Notes | Ledger` 切换；Ledger 页按 State／Open／Conventions 三组，每组一个凸起框，每条 `#编号`、标题、淡色两行正文 |
| 甲 点开 | 点一条就地展开：正文全文（Markdown），下面 `Added／Updated／By／Entry` 四行；只读，没有按钮 |
| 乙 同一页收成一行 | 不加切换，`Project ledger` 一行放在 Next session 上面，点开原地列出 |
| 空账 | `No ledger entries yet`，说明谁会写；不当出错 |
