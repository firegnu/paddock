# 任务 P2-1：Agents 面板按 Saddle 做全

2026-10-05，paddock/main 自己做。
类型：功能变更
依据：DESIGN §11 第 1 项。外观不比 Saddle 差；本轮只做面板的显示、折叠、排序，不做操作 agent（New、Stop）、Attention、Tasks、Search。

## 参照

- 用户 10-05 提供的日常 Saddle 截图（未入库）：Agents 面板的实际样子。
- Saddle `df1c727` 的实现（只读）：`src/ui.rs` 中 Agents 面板的绘制，`src/agents.rs`、`src/git.rs`。
- Saddle `docs/设计稿/agents-panel-3a/Agents Panel Spec.md`：列宽、颜色、截断、折叠、动画的规格。与 Saddle 实际实现不一致时，以实际实现和截图为准。

## 要做的

1. **头部**：`Agents · <总数>`，下方一条分隔线。（Saddle 头部的 Attention、Tasks、`⋯` 属于后面的阶段，本轮不放。）
2. **分组标题**：`name/` + 横线填满 + `(数量)` 右对齐；分组之间空一行。
3. **每个 agent 的多行信息**，按 Saddle 的行规则和显示条件：
   - 第一行：状态点（working 时动画）、名字、哪家（`✳ claude`、`>_ codex` 等，按 Saddle 的颜色）、强度格（有 `effort` 标签时）、状态（working 时前面有转动的点阵）、时间（右对齐；本窗口正接入的 agent 前面有 `⦿`）。
   - 会话标题（与组名相同时不显示）。
   - 当前活动（如 `DOING Bash · 4m`）。
   - git 行：分支、领先或落后的提交数和对照分支、改动行数 `+a -d ?u`（放不下时整组换到下一行右对齐）。
   - 目录（超宽时从左侧截断，保留末段）。
   - `实例号 · ATT n · VIA 来源`。
   - 各列跨条目对齐；文字用终端同款等宽字体，按格子对齐。
4. **选中样式**：当前接入的那个 agent 用 Saddle 的选中底色和左侧色条；未选中的左侧是暗色竖线。
5. **折叠与排序**：
   - 折叠：未选中的只显示第一行，选中的完整展开；默认在 agent 多于 5 个时折叠（同 Saddle）。
   - 排序：默认按状态（需要人处理的在前），可切换成按名字。
   - 底栏：`s Sort`、`z Fold`，用鼠标点击切换（键盘快捷键留到菜单栏那一步）；左边显示 `[Attached]`（同 Saddle）。
6. **git 数据**：迁入 Saddle `src/git.rs`（文件开头注明来源，做法同 DESIGN §10），后台定时采集各 agent 目录的 git 摘要；`agents::Panel` 恢复 M0 时去掉的 git 部分。Saddle 对应的 git 测试一并迁入（`tests/git.rs`、`tests/agents.rs` 中的 git 用例），Python 假程序改成 shell。
7. **颜色**全部取自主题（Saddle 的 `agents_*`、`agent_*`、`claude`、`codex` 等字段），三套主题下都要正常。

## 怎么算做完

- 和用户的 Saddle 截图对照，面板的结构、信息、对齐、颜色不比 Saddle 差。截图给用户看，用户认可。
- 点击接入、新 shell 照旧可用。
- 现有测试照样通过，迁入的 git 测试通过；新增测试覆盖：行的显示条件、git 改动换行规则、目录截断、折叠、排序。`cargo clippy --all-targets -- -D warnings` 无警告。

## 不要做

- 不做 New、Stop、Attention、Tasks、Search 和键盘快捷键。
- 不改终端窗格、标题栏（标签页和窗格边框是 P2-2）。
- 不读 Corral 内部状态目录；git 采集照 Saddle 的做法，只读本地数据、不联网、不改仓库。
- 不改 Saddle 仓库。

## 完成记录

2026-10-05，paddock/main。用户看截图后认可：“我觉得还可以”。开工前用户补充：“不一定复刻，但是信息不能少，而且我觉得可以做的更加漂亮，毕竟不再被困在tui中了。”

- **做了什么**：
  - 迁入 Saddle `df1c727` 的 `git.rs`、`command.rs`（`corral.rs` 恢复为原样，不再内嵌 command），`agents.rs` 恢复 git 部分；迁入 `tests/git.rs`、`tests/git_env.rs` 和 `tests/agents.rs` 的 git 用例。
  - 新增 `card.rs`：按 Saddle `ui.rs` 的规则算出每个 agent 要显示的全部信息（状态点与动画、哪家、强度、状态、时间与 `⦿`/`•`、标题、活动行、git 行与改动换行、目录截断、实例号·ATT·VIA），可脱离窗口测试。
  - 重写 `sidebar.rs`：GPUI 原生卡片；头部总数；分组标题带横线与计数；选中卡片色条与底色、悬停效果；名字和标题用系统字体，git、目录、实例号等用终端字体；强度改为三格信号图标；底栏 `s Sort`、`z Fold`、`＋ New shell` 可点；后台每 5 秒采集 git。
  - 侧栏默认宽度 240 → 380 pt（窄于 320 时紧凑显示）；`app/README.md` 同步。
- **验证了什么**：90 项测试通过（新增面板模型测试 9 项、迁入 git 测试 9 项）；clippy、fmt 通过。用假 corral 提供的合成 agent（目录指向本机仓库）启动 release 版并只截本窗口，核对展开、选中、git、ASK/DOING 显示；截图给用户看过，未入库。
- **拿主意的地方**：不放 Saddle 底栏的 `[Attached]`（TUI 键盘焦点提示，GUI 里不需要）；强度用三格信号图标代替点阵字符；默认宽度改为 380。
- **没做的事**：New、Stop、Attention、Tasks、Search、键盘快捷键；真实 agent 上的点击操作留给用户体验。
