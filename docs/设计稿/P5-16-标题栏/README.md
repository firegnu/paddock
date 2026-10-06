# P5-16 标题栏样稿

用户 10-06 看过，按推荐定稿（见 `docs/DESIGN.md` §13 P5-16）。原稿在 claude.ai 私有画布的「P5-16 侧栏头部与标题栏分割线」页，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里），读 HTML、内联样式和 `Main.dc.html` 末尾 `renderVals()` 即可；agent、终端内容都是示意。

`Main.dc.html` 是带参数的窗口（`head` 取 a / b / now，`line` 取 term / full / pet / none，另有 `collapsed`、`pet`、`narrow`、`fullscreen`），其余文件只是给它换参数。

| 文件 | 内容 | 定了没有 |
| --- | --- | --- |
| `Main.dc.html` | 乙：收起图标紧跟红绿灯，线一 | **采用** |
| `HeadA.dc.html` | 甲：照原顺序 Agents · 铃铛 · 收起 | 未采用 |
| `Now.dc.html` | 现状，对照 | — |
| `RailB.dc.html` | 乙 收起：展开图标留在标题栏，窄条顶上只剩铃铛 | **采用** |
| `RailA.dc.html` | 甲 收起：展开图标和铃铛留在窄条顶上 | 未采用 |
| `LineTerm.dc.html` | 线一：只在终端一侧，竖分隔线通到顶 | **采用** |
| `LineFull.dc.html` | 线二：整条横贯 | 未采用 |
| `LinePet.dc.html` | 线三：只在宠物那段 | 未采用 |
| `PetOff.dc.html` | 宠物关掉，线保留 | **采用** |
| `Narrow.dc.html` | 侧栏最窄 220pt，铃铛收成小图标 | **采用** |
| `Fullscreen.dc.html` | 全屏，没有红绿灯，头部靠左 12pt | **采用** |

颜色取自 Dune 主题，实现时一律从主题取色。界面文字一律英文。
