# P5-94 Integrations 样稿

用户 10-14 要 Settings 里能一键配置 agent，像 herdr 的 integrations 标签（限 Claude Code、Codex、pi、omp）。主控按 ranch R4 的 `corral integration status` 出了样稿，用户：“样稿可以，写任务文件吧”（见 `docs/任务/P5-94-Settings里一键配置agent.md`）。

原稿在 claude.ai 私有画布「Integrations样稿」，这里是同一份源文件（`.dc.html`，引用的 `support.js` 不在这里；画布上三张叫 `Main`、`Confirm`、`States`）。配色取 Lagoon 的近似值，实现时一律从主题取色。三张都是静态的，没画悬停。

| 文件 | 内容 |
| --- | --- |
| `1-Agents.dc.html` | Agents 页：corral 一节下面加 Integrations。每家一行：名字和版本、Status（`Built in`）、Skills（`Up to date` 或 `Update` 加一句哪个技能变了）。Codex、pi、omp 共用 `~/.agents/skills`，所以一起显示 Update |
| `2-Confirm.dc.html` | 点 pi 的 Update 后的确认框：写哪个文件、这个位置还有哪几家也读 |
| `3-States.dc.html` | 第一次用（技能都是 Install、没装的 agent 变淡写 Not found）、读的时候 `Checking…`、corral 找不到 |

样稿上的出入，以任务文件为准：
- 样稿左边的页面列表没画图标；实现时照现在的 Settings。
- 版本号、路径是本机 10-14 的真实值。
