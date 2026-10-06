# P5-18 agent 卡片样稿

用户 10-06 看过，定第二版「收件箱」（见 `docs/DESIGN.md` §13 P5-18），并要求“信息不能丢”。原稿在 claude.ai 私有画布「P5-18 agent 卡片信息精调」页，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）；agent、回复内容都是示意。配色取自 Lagoon，实现时一律从主题取色。

| 文件 | 内容 |
| --- | --- |
| `Inbox.dc.html` | **采用**：收件箱样式，收起时（参数 `open` 取 none / work / idle） |
| `InboxWork.dc.html`、`InboxIdle.dc.html` | 同上，展开干活中的、空闲的 |
| `Now.dc.html` | 现状（`Main.dc.html` 的 `look="now"`），对照用 |
| `Main.dc.html`、`OpenWork.dc.html`、`OpenIdle.dc.html` | 第一版（只理顺信息），未采用 |

样稿里有几处信息没画全（第三行位置行、强度、Instance、等你卡片上的 Open），以 DESIGN 和任务文件为准。
