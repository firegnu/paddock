# P5-13b Changes 标签样稿

用户 10-06 看过：“可以没问题的，派出去做吧。”（见 `docs/DESIGN.md` §13 P5-13b）。原稿在 claude.ai 私有画布「P5-13b Changes 标签」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）；读 HTML、内联样式和末尾脚本里的数据即可。文件名、代码、行号都是示意。配色取自三套预置主题，实现时一律从主题取色。界面文字一律英文。

| 文件 | 内容 |
| --- | --- |
| `Main.dc.html` | 主窗口，右侧栏开在 Changes，宽 500，Uncommitted，窄面板 unified；面板本身由 `Panel.dc.html` 画 |
| `Panel.dc.html` | 面板本体：顶部两行（跟随的 agent、分支、总增删；范围切换、文件数、全部折叠）、文件索引、逐个文件的 diff（贴顶文件头、hunk 头、“N unchanged lines”、整行底色＋改词深一档、语法高亮）；参数 `theme`、`scope` |
| `Wide.dc.html` | 加宽后：左边文件树，右边左右对照（split），顶部多一个 Unified／Split 切换 |
| `Themes.dc.html` | Dune、Tide、Lagoon 三套并排 |
| `States.dc.html` | 没有改动、不在 git 仓库里、特殊文件（改名、删除、二进制、太大）、git 出错 |
