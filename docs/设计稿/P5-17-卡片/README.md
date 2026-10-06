# P5-17 窗格卡片风格样稿

用户 10-06 看过，按主控建议定（见 `docs/DESIGN.md` §13 P5-17）。原稿在 claude.ai 私有画布「P5-17 窗格卡片风格」页，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里），读 HTML 和 `Main.dc.html` 末尾 `renderVals()` 即可；agent、终端内容都是示意。

`Main.dc.html` 是带参数的窗口（`look` 取 card / now，`panes` 取 1 / 2 / 3，另有 `collapsed`、`right`、`gap`、`radius`），其余文件只是给它换参数。

| 文件 | 内容 |
| --- | --- |
| `Main.dc.html` | 卡片：左右分屏（采用） |
| `Card1.dc.html` | 卡片：单窗格 |
| `Card3.dc.html` | 卡片：三个窗格，当前之外压暗 |
| `CardRail.dc.html` | 卡片：侧栏收起成窄条 |
| `CardRight.dc.html` | 卡片：右侧栏打开 |
| `Now1.dc.html`、`Now2.dc.html` | 对照：P5-16 之后的样子 |

样稿里窗格标题画了四个按钮（往右分、往下分、放大、关闭），实现沿用现有的三个（分屏、放大、关闭）。颜色取自 Dune 主题，实现时一律从主题取色。界面文字一律英文。
