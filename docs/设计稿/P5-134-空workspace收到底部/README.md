# P5-134 空 workspace 收到底部 样稿

用户 10-17：“如果我有一个workspace中有好几个agent，但是我如果把这些agent都关闭的话，界面上就会留一个空的workspace很难看。但是如果选择了关闭wokspace，下一次启动的时候就得重建workspace。”主控给了甲（空了自动收到底部）、乙（手动 Close）、丙（只画小）三种做法，用户：“甲的话先出一个设计图”；看过样稿：“选甲·二，规则都按你的来，附图也要，写任务文件吧”。

原稿在 claude.ai 私有画布「P5-134 空 workspace 收到底部样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）。配色照 P5-91 样稿的深色主题近似，实现时一律从主题取色；侧栏宽 340 是示意。workspace 名、仓库名、卡片上的字都是占位的。

| 文件 | 内容 |
| --- | --- |
| `Main.dc.html` | 现在：空的 Fine-tuning 展开着三行 `No agents`，空的 client-demo 收着也占一行 |
| `A1-Shelf.dc.html` | 甲·一：列表末尾一行 `Idle` 加胶囊。没采用 |
| `A2-Group.dc.html` | **采用**：甲·二，列表末尾细线下一个 `Idle workspaces 2` 小组，每个空 workspace 一行（层叠图标、名字、右边 `3 repos`，鼠标停上去出 `⋯`） |
| `A3-Open.dc.html` | **采用**：点了底下的 client-demo 后，它回到上面展开（`demo-app No agents`，鼠标停上去右边 `+`），底下只剩 Fine-tuning |
| `B-IdleRepos.dc.html` | **采用（附）**：有 agent 的 workspace 里，没 agent 的仓库合成一行 `datasets, eval-kit  No agents`，点开再逐行显示；这张里 `Idle workspaces` 是收着的样子 |

样稿上没画的：合成那一行点开后的样子（就是现在一行一个仓库）；浅色主题；左侧栏收成窄条；鼠标停在小组标题上的样子。
