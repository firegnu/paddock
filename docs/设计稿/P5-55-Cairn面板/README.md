# P5-55 Cairn 面板样稿

用户 10-08：“paddock写cairn的面板”；10-10：“那就开始P5-55 这个任务，来确定cairn工作正常。咱们俩先确定一下scope。之后再委派”。范围和主控的建议逐条谈过，用户：“都按你的建议来，出样稿吧。”看过样稿后：“加一句提示命令的文字吧，然后写任务文件”（见 `docs/任务/P5-55-cairn面板.md`）。原稿在 claude.ai 私有画布「P5-55 Cairn 面板」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里）。配色取深紫主题的近似值，实现时一律从主题取色；仓库名、记录内容、记录编号都是示意。

| 文件 | 内容 |
| --- | --- |
| `Main.dc.html` | 已采用，有记录（平时的样子）：标签行第四个是 `Cairn`；仓库名、分支、`Adopted`；一行 hook 状态（Claude、Codex 各自装没装，右边 `N uncollected`）；下面是下次会话会被注入的全文，按 Markdown 画（`###` 画成淡的分节小标题加细线，`##` 画成加粗小标题） |
| `NotAdopted.dc.html` | 有 cairn、hook 装了、这个仓库没采用：一段说明、仓库路径、`Adopt…` 按钮 |
| `Confirm.dc.html` | 点了 `Adopt…`：面板里的一张确认卡片，写明立即生效、派出去的 agent 不受影响、怎么撤回；`Cancel`／`Adopt` |
| `States.dc.html` | 其余六种：已采用还没记录（顺带画了只装一家 hook 的状态行）；有存了没收走的（`2 uncollected` 变黄）；hook 一家都没装（一句说明加两条安装命令的文字，没有按钮）；不在 Git 仓库里（不给 Adopt）；没装 cairn；命令出错 |
