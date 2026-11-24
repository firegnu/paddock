# P5-201 workspace 侧栏精修 样稿

用户 11-24 发来四张左侧栏截图（没有 agent、Idle 行的菜单、点开放上来的空 workspace、有 agent 的 workspace 加 Idle）：“左侧边栏的workspace的各种情况能否再精细设计一下。我现在的碰到的问题主要是idle的workspace一多，我就不好找我要打开的了，我觉得应该有搜索以及加一个星星那种功能。还有其他的你觉得精细调整要有的都可以设计。”主控先画了一版功能（A），用户：“都按照你的建议来，关于workspace的相关的ui能不能再做的漂亮一些？”主控又画了二版画法（B），用户：“三处都按你的来，写任务文件吧”。

原稿在 claude.ai 私有画布「Workspace 侧栏精修样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）。配色照截图近似，实现时一律从主题取色；侧栏宽 368 是示意。卡片只画了个色块，卡片本身不改。

功能以 A 为准，画法以 B 为准；两者冲突时照 B。

| 文件 | 内容 |
| --- | --- |
| `A-Main.dc.html` | 一版：Idle 托盘标星的在前、细线下按最近用过排；标题右边放大镜；行左边留星标格，鼠标停上去出空心 ☆，行尾 `+`、`⋯`；仓库名放不下写 `canter-test-python +2` |
| `A-Filter.dc.html` | 一版：过滤中，输入 `can`，名字和仓库名都匹配，命中的字强调色，`3 of 9`，第一行选中，底部快捷键 |
| `A-Folded.dc.html` | 一版：Idle 收起后标星的几行还露着（B 改成胶囊）；上面标星的 workspace 名字后带小 ★ |
| `A-Raised.dc.html` | 一版：点开放上来的空 workspace，虚线边、`No agents` 小签、`Put back`、每个仓库一行常显的 `New Agent`（B 改成仓库胶囊）。底部 `… 3 more`（Idle 限高）**不采用** |
| `A-Palette.dc.html` | ⌘P 里多一组 `Workspaces`。图上 `⌘↵ new agent in it` 和 Commands 里的 `New Agent in My-Harness…` **不采用**，只做 ↵ 打开 |
| `B-Main.dc.html` | **采用**：字母签（按名字自动配色）；在用的托盘左上角一层签色的淡光、标题 签·名字·★·`2 agents`·箭头在最右；仓库小标题个数做成小签、后面细线连到状态细条；合成行 `cairn · ranch  No agents ›`；Idle 行两行（名字 / 仓库名）、右边上次在用的时间、`STARRED`／`RECENT` 两段小标签；`global-mesh` 那行是鼠标停上去的样子 |
| `B-Folded.dc.html` | **采用**：Idle 收起后标星的变成一排带签的胶囊（`global-mesh` 是鼠标停上去的样子）；收起的 workspace 一摞也带签色的淡光 |
| `B-Raised.dc.html` | **采用**：放上来的空 workspace：虚线边、`Nothing running here`、每个仓库一颗 `+ 仓库名` 胶囊（`paddock` 是鼠标停上去的样子）、右上 `Put back` 和 `⋯` |
| `B-Filter.dc.html` | **采用**：过滤框带强调色描边，选中行强调色淡底和 `↵` 小签，底部居中快捷键提示 |
| `B-Light.dc.html` | **采用**：浅色主题下的样子（字母签换成浅底深字） |

样稿上没画的：没有任何 agent 时；没有标星的 Idle；左侧栏收成窄条（不变）；远程视图（不变）。
