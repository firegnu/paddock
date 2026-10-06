# P5-13 右侧边栏样稿

用户 10-06 看过并认可（见 `docs/DESIGN.md` §13 P5-13）。原稿在 claude.ai 私有画布的「P5-13 右侧边栏」页，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里），读 HTML、内联样式和末尾 `renderVals()` 的数据即可；文件、diff、网页内容都是示意。

| 文件 | 内容 |
| --- | --- |
| `RightIcons.dc.html` | 标题栏右端：分屏（重画：两半加 `+`）· Search · 右侧栏开关；三个图标放大对照，左侧栏收起图标是右侧栏的镜像 |
| `Right.dc.html` | 主窗口组件，`start` 取 closed / diff / browser；打开时面板在右、分割线、顶部标签、加宽和关闭按钮 |
| `RightClosed.dc.html` | 收起时 |
| `RightBrowser.dc.html` | Browser 标签（内容这一轮不做） |

这一轮只做外壳（用户 10-06：“diff和浏览器这一轮先不做，先把你的设计做出来”）；Changes、Browser 两个标签里的内容以后再做。颜色取自 Dune 主题，实现时一律从主题取色。界面文字一律英文。
