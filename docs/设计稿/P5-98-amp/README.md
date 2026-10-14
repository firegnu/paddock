# P5-98 amp 集成样稿

ranch R6 接入 amp 后，主控按 `corral integration status` 的 `hooks:"global"` 和 `install` 出了样稿。用户 10-14 看过：“选B，样稿可以，开始吧”（见 `docs/任务/P5-98-集成amp.md`）。

原稿在 claude.ai 私有画布「amp集成样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里；amp 图标引用的是画布上传的图片 `/_blob/…`，图片本身不在仓库）。Settings 的配色取 Lagoon、New Agent 取 Dune 的近似值，实现时一律从主题取色。都是静态的，没画悬停。

| 文件 | 内容 |
| --- | --- |
| `1-Main.dc.html` | Agents 页的 Integrations 多一行 amp：插件已装，绿色 `Installed` 加安静的 `Uninstall`；卡片下的说明改成三句 |
| `2-States.dc.html` | amp 一行另外四种样子：没装（Install）、旧了（Update 加 `plugin changed`）、不是 corral 的（Not managed）、找不到 amp |
| `3-Install.dc.html` | 点 Install 的确认框：写哪个文件、只在 corral 启动的 amp 里起作用、每个 amp 多一个闲置进程、已开的要重开 |
| `4-Uninstall.dc.html` | 点 Uninstall 的确认框 |
| `5-NewAgentA.dc.html` | A：New Agent 选 amp，Prompt 框变灰（未采用） |
| `6-NewAgentB.dc.html` | B：Prompt 照常填，amp 空闲后由 paddock 送进去；插件没装时的黄色提醒和 `Open Settings`（**采用**） |
