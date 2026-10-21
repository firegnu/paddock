# P5-164 Idle workspaces 改成底托盘 样稿

用户 10-21 嫌左侧栏底下的 `Idle workspaces` 样子有问题：“我觉得这个功能对我帮助很大，但是总感觉这个ui的设计有点问题。”主控画了甲（收紧成清单）、乙（休眠托盘）、丙（底托盘）三种，用户：“选丙，重复那行也一起改，写任务文件吧”。

原稿在 claude.ai 私有画布「Idle workspaces 改版样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里），只收了现在的样子和采用的丙。配色照截图近似，实现时一律从主题取色；侧栏宽 368 是示意。仓库名（`mesh-core`、`plans` 等）是占位的。

| 文件 | 内容 |
| --- | --- |
| `Main.dc.html` | 现在：细线下 `Idle workspaces 3`，一行一个闲着的 workspace（灰色层叠图标、名字、右边 `2 repos`，后面给 `⋯` 留着空位）；上面托盘里合成行展开后 `cairn, ranch No agents` 下面又列一遍 `cairn`、`ranch`，没缩进 |
| `C-Shelf.dc.html` | **采用**：闲着的 workspace 装进一个更淡的托盘，标题 `Idle 3` 和 workspace 标题同一个样子；每行名字照仓库小标题的样子，右边淡字写它的仓库名，`longterm-plan` 那行是鼠标停上去的样子（变亮、行尾 `⋯`）。上面托盘里合成行展开的样子也改了：标题成 `No agents 2`，下面的仓库名缩进对齐、不再各带 `No agents`，`ranch` 那行是鼠标停上去的样子（右边 `+`） |

样稿上没画的：`Idle` 托盘收起的样子；没有任何 agent 时；浅色主题；左侧栏收成窄条。
