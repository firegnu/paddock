# P5-89 Workspace 样稿

用户 10-14：“现在的左侧的agents是按照repo来列的。我想记一个所谓的workspace的概念。……如果理解了我的意思的话，就先设计。出设计稿给我。注意下兼容现在的样子（以及前两天修复的委派出去的agent和主控开在同一个repo下）。先设计吧。”（全文见 `docs/任务/P5-89-workspace.md`）。看过第一版后加了两条（仓库能收起、能只加某几个 agent），主控出了第二版；用户：“都按你的建议来，写任务文件吧。”画法采用托盘（`1-Tray`），`1b-Flat` 没采用。

原稿在 claude.ai 私有画布「P5-89 Workspace 样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里；画布上六张叫 `Now`、`Main`、`B-Flat`、`New`、`RepoMenu`、`WorkspaceMenu`）。配色照 P5-74 样稿里的侧栏近似，实现时一律从主题取色；侧栏宽 340 是示意。都是静态的；卡片上的文字、workspace 的名字（`Agent tools`、`Fine-tuning`）是占位的。

| 文件 | 内容 |
| --- | --- |
| `0-Now.dc.html` | 现在的样子：按仓库分组。没建 workspace 时一点不变 |
| `1-Tray.dc.html` | 有一个 workspace（主控推荐的画法）：workspace 是一个浅底的托盘，里面照旧是仓库小标题和卡片；没进 workspace 的仓库在下面，和现在一样。托盘里 `saddle` 一行是“在 workspace 里、现在没有 agent 的仓库” |
| `1b-Flat.dc.html` | 同样的内容，另一种画法：不加底，workspace 一行标题，下面的仓库缩进、左边一条细线，和没进 workspace 的仓库之间一条分隔线 |
| `2-New.dc.html` | 新建 workspace：起名字，勾选现在开着的仓库（整个仓库），或点开某个仓库只勾其中几个 agent；`Choose folder…` 加一个现在没开的 |
| `3-RepoMenu.dc.html` | 鼠标停在没进 workspace 的仓库小标题上，出一个小按钮，点开选加进哪个 workspace 或新建一个；上面那个 workspace 是收起的样子 |
| `4-WorkspaceMenu.dc.html` | 鼠标停在 workspace 标题上出 `⋯`，点开：改名、加仓库、去掉这个 workspace；下面第二个 workspace 是收起的样子 |
| `5-RepoFold.dc.html` | 仓库也能收起（用户 10-14 加的第 1 条）：`paddock`、`jb-finetune` 收成一行，名字后面一个小箭头，右边的状态细条照旧；`global-mesh` 是鼠标停在展开的仓库小标题上的样子；收起的仓库里有正在显示的 agent 时名字后面多一个强调色小圈 |
| `6-Split.dc.html` | 只把某几个 agent 加进 workspace 之后（用户 10-14 加的第 2 条）：`codex-1` 在 workspace 里，同一个仓库的另一个 agent 留在下面，仓库名出现两次；展开的卡片详情里多一格 `Workspace`，点开选加进哪个 |

画布上后两张叫 `RepoFold`、`Split`。

样稿上没画的：workspace 里的仓库小标题悬停时的菜单（`Move to …`、`Remove from <workspace>`）；左侧栏收成窄条时的样子；改名时标题就地变成输入框。
