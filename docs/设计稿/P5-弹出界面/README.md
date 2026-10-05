# P5-4 弹出界面 · P5-10 底部按钮 · P5-11 分割线与收起 样稿

用户 10-06 看过后定下（见 `docs/DESIGN.md` §13）。原稿在 claude.ai 的私有画布上（页面「P5-4 弹出界面 · P5-10 · P5-11」），这里是同一份源文件，供实现时查颜色、尺寸和结构。文件是画布用的 `.dc.html`（引用的 `support.js` 不在这里），直接读 HTML、内联样式和末尾 `renderVals()` 里的数据即可；终端文字和 agent 名都是示意。

| 文件 | 内容 |
| --- | --- |
| `Shell.dc.html` | 主窗口组件，下面五个都是它的不同状态（`start` 取 plain / menu / rail / newtab / split / attention）：分割线、侧栏收起按钮、底部按钮和菜单、收起后的窄条、三个弹出框都在这一个文件里 |
| `ShellMenu.dc.html` | P5-10：底部按钮点开的菜单，向上弹出 |
| `ShellRail.dc.html` | P5-11：侧栏收起成 52pt 窄条，悬停一个 agent 时的说明 |
| `ShellAttention.dc.html` | P5-4：Attention 列表，挂在铃铛下 |
| `ShellNewTab.dc.html` | P5-4：新标签选择框，挂在 `+` 下 |
| `ShellSplit.dc.html` | P5-4：分屏框，挂在窗格的分屏按钮下，方向和内容一步选完 |
| `NewAgent.dc.html` | P5-4：New Agent 窗口 |
| `Settings.dc.html` | P5-4：设置窗口（General、Colors 两页画了内容） |
| `About.dc.html` | P5-4：About 窗口 |

与样稿不同、以用户决定为准的地方：
- 底部按钮只放图标，不写 `Actions`，旁边也不写当前排序（用户 10-06：“只放图标”）。
- 收起侧栏加快捷键 ⌘B（用户 10-06：“要”）。

颜色取自 Dune 主题；实现时一律从主题取色，不写死这些十六进制值。界面文字一律英文。
