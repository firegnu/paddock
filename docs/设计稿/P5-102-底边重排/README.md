# P5-102 New Agent 底边重排样稿

用户 10-15 嫌底边那排模型名长时 Controller 被挤到第二行，看过两行、带小标题、Controller 挪右上角三版后说“我总觉得这个设计还是不够优雅，主要是ui，不是逻辑”；主控提了去边框、分主次两个方向，用户：“一和二合起来，出样稿吧”；看过丁：“可以，照丁做吧”（见 `docs/任务/P5-102-New Agent底边一行放不下.md`）。

原稿在 claude.ai 私有画布「New Agent底边重排样稿」，这里是丁的三张源文件（`.dc.html`，引用的 `support.js` 不在这里）。图标是色块占位，配色取用户截图里主题的近似值，实现时一律从主题取色。

| 文件 | 内容 |
| --- | --- |
| `D1-Codex.dc.html` | Codex 普通 agent：`[Codex ▾ │ gpt-6.1-sol xhigh ▾] 📁 paddock … [右半填的分屏图标] [皇冠] │ </> (↑)` |
| `D2-Claude.dc.html` | Claude 主控：皇冠是强调色、带淡底；打开位置是 This pane（整格填） |
| `D3-Hover.dc.html` | 悬停：模型那半边亮一档；打开位置图标上方提示 `Opens in Split right` |
