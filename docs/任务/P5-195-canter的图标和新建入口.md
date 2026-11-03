# 任务：canter 的图标，New Agent 能选 canter（默认 --yolo），Agent shell 接管 canter

2026-11-03，paddock/main 交给 paddock/dev-canter（Claude Code，`opus[1m]` / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.5）、拿不准、拿不准；照 P5-179 同类活定不要交叉审查、改行为）
类型：功能变更
依据：canter/main 按用户要求交来的需求单（`/tmp/canter-p2-193/paddock-需求单.md`）第 1～3 条；本轮做不依赖 corral 新接口的部分，用假 corral 测试。第 4、5 条（暂停、沙箱按能力显示，Integrations 的 canter 一行）等 ranch 定了 corral 的接口另开一件（P5-196），本轮不做。照 P5-179 接 kimi 的做法接 canter，不另做花样。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- 11-03 看完 canter 的调研报告后：「其他我都没有异议，可以按照你的建议走」；「也就是说这一版不做pause和fence，其他都会做，包括paddock相关的。」
- 11-03：「如果其他agent委派出canter，要使用--yolo开出。」之后：「paddock 也加」。
- 11-03：「我的意思是功能不能反向依赖，而不是开发顺序。以后canter要能独立运行的」。
- 需求单第 1～3 条（canter/main 写，经用户确认的范围）：
  1. 新建 agent 的入口能选 canter（和其它几家并列），能识别 canter 的启动命令；模型先沿用 canter 自己的配置，不必在 paddock 里选。在 paddock 里新建的 canter 默认带 `--yolo` 开。
  2. canter agent 的终端接管照现有方式（attach）；shell 自动接管的名单认得 canter。
  3. canter 有自己的图标，和其它几家区分开。

## 先读
- 仓库根的 `CLAUDE.md`（规矩，尤其「独立编译目录」「桌面窗口测试」「验证」）。
- `docs/任务/P5-179-kimi的图标.md`：kimi 是怎么接的，canter 照它做；对应代码 `git show 626c08e`。
- `docs/DESIGN.md` 里 P5-61（Agent shell）、P5-179（集成 kimi）那几条。
- `canter --help`（本机已装，0.1.0 · 16fb98c）：子命令和选项以它为准。canter 没有官方图标。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-195-canter`，分支 `p5-195-canter`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-195-canter`（已从 `.target/main` 克隆）。命令在 `app/` 下跑。
- 会动的大致是：`kind_icon.rs` 和 `app/assets/kinds/`（含 README）、`preset.rs`、`card.rs`、`settings.rs`（品牌色）、`new_agent.rs`、`new_agent_view.rs`、`agent_shell.rs`、测试、`docs/DESIGN.md` 和本文件。以实际需要为准。

## 要做的

### 一、图标
1. 仓库 `app/assets/kinds/canter.svg`：paddock 自己画的单色剪影，风格和现有几个一致、小尺寸看得清。canter 是用户自己写的 agent，没有官方标志，README 记为 “Original drawing for paddock.”；README 认的名字里加 `canter`。
2. 本机 `~/.config/paddock/icons/canter.svg|png` 有时画它（和其它几家同一套规则），没有时画仓库剪影。
3. 主题多一项 `canter` 品牌色：每套预设各给一个和另外七家分得开的颜色（`every_theme_tells_the_agent_kinds_apart` 要过），Settings 的 Colours 页可改。

### 二、New Agent
4. 种类多一个 canter，排在 kimi 后：标签 `canter`，命令 `canter --yolo`；没有模型和强度（用 canter 自己的配置）；预设能存。
5. 认得 canter 的启动命令：手改的命令（`canter`、带路径的 canter、带别的参数）读回成 canter；手动删掉 `--yolo` 照用户改的起，和 codex 去掉 `--yolo` 一样。
6. Prompt 可选，行为和 claude、codex 一样（空着起成 idle）；首句交给 `corral start --prompt`，放进 canter 的位置参数是 corral 的事，paddock 不自己拼。canter 不需要装插件或钩子，不出钩子提醒。

### 三、Agent shell
7. 替身多一个 `canter`：在 Agent shell 里敲 `canter …` 经 corral 托管，照用户敲的参数起，不自己加 `--yolo`。
8. 直接跑真 canter、不经 corral 的：非终端、`-h`／`--help`、`-V`／`--version`，以及 `canter --help` 列的管理和一次性子命令（现在是 `serve`、`service`、`upgrade`、`rollback`、`usage`、`exec`、`stats`、`mcp`，以你跑出来的为准）。交互的 `canter`、`canter <首句>`、`--resume`／`-r`、`-c`／`--continue`、带 `-m`、`--tag`、`--yolo` 等选项的交互启动，经 corral 托管。

### 四、文档
9. `docs/DESIGN.md`：P5-61 补 canter；加一条「接 canter（P5-195）」，写用户原话、定下的做法，并写明第 4、5 条留给 P5-196。

## 怎么算做完
验收照需求单第 1～3 条：
1. 新建 agent 的入口能选 canter（和其它几家并列），能识别 canter 的启动命令；模型沿用 canter 自己的配置；新建的 canter 默认带 `--yolo`。
2. shell 自动接管的名单认得 canter（终端 attach 照现有方式，不改）。
3. canter 有自己的图标，和其它几家区分开。
4. 现有七家一点不变。
- 验证只做这些：给新逻辑加针对性单元测试（种类列表和顺序、命令读回 canter、预设存读、替身直通和托管、图标名、品牌色分得开）；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 真实 canter 起成 corral agent 要等 ranch 让 corral 认得 canter，由主控合并安装后和用户、canter 主控、ranch 主控一起联调，你不做。

## 不要做
- 只用合成数据和假 `corral` 脚本测试；不开真实 canter 或任何 corral agent，不碰 `corral ls` 里已有的 agent（不 stop／send／keys）。
- 可以跑 `canter --help`、`canter <子命令> --help` 看参数；不跑 `canter` 交互启动、`serve`、`service`、`upgrade`、`rollback`、`exec`，不碰用户正在跑的 canter 服务端，不读 `~/.canter` 和 canter 的状态目录。
- 不改 canter、ranch、Saddle 仓库。
- 不做暂停、沙箱、Integrations 的 canter 相关改动（P5-196 的事）；不做远程机器 New Agent 的 canter（没定，主控在问用户）；不加 Settings › Instructions 里 canter 的说明文件。
- 不按名字批量杀进程（`pkill -f paddock` 这类）；停自己起的进程用启动时记下的 PID。
- 截图、起测试窗口照 CLAUDE.md「桌面窗口测试」；不用脚本模拟按键或鼠标。
- 碰到要拿主意的事，停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-195-canter` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

### 做了什么
- 图标：`app/assets/kinds/canter.svg` 是自画的单色马蹄铁（开口朝下），和另外七家的形状都不像；README 记 “Original drawing for paddock.”，认的名字加 `canter`。`kind_icon.rs` 认 `canter`，本机 `~/.config/paddock/icons/canter.svg|png` 有时画它，没有时画剪影。
- 品牌色：主题多 `canter`（`[colors]` 键 `canter`，Settings › Colours 的 Agent kinds 组里可改，卡片按它着色）。另外七家没用绿，每套取自己的绿；Rosé Pine、Rosé Pine Dawn 调色板里没有绿，取灰绿 `#b0c88f`、橄榄绿 `#5f7f3e`。
- New Agent：`Tool::Canter` 排在 kimi 后，标签 `canter`，命令 `canter --yolo`（`CANTER_COMMAND`），没有模型和强度、没有插件或钩子提醒，Prompt 可选（空着起成 idle，有字走 `corral start --prompt`），预设能存。手改的 `canter`、带路径的、带别的参数的读回成 canter；删掉 `--yolo` 照改的起；canter 的 `-m` 不读成模型、不打 `model=` 标签。
- Agent shell：替身多 `canter`，照敲的参数经 corral 起，不自己加 `--yolo`。`-V` 和 `canter --help`（0.1.0 · 16fb98c）列的 `serve`、`service`、`upgrade`、`rollback`、`usage`、`exec`、`stats`、`mcp` 直通真程序（非终端、`-h`／`--help`、`--version` 本来就直通）。
- DESIGN：P5-61 补 canter；§13 加「接 canter（P5-195）」，写明需求单第 4、5 条留给 P5-196。

### 验证了什么
- 新加单元测试：替身直通和托管（agent_shell，先跑到 RED 再实现）；种类顺序、`canter --yolo` 和有无首句的 corral 参数、手改命令读回 canter 且照原样起、删 `--yolo` 保留、预设存读后表单对得上（new_agent）；剪影文件名、本机原图按名字读到（kind_icon，先 RED）；品牌色分得开、卡片着色、Colours 分组和计数（preset、card、settings）；`empty task starts it idle` 名单加 canter。New Agent 和颜色的测试和新变体、新字段一起写，之前编译不过，没有单独看到 RED。
- `app/` 下 `cargo test --all-targets`（1102 个全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都过。
- 用 `rsvg-convert` 把剪影渲染成 128px 和 14px 看过，小尺寸认得出是马蹄铁。没开 paddock 窗口看界面。

### 拿主意的地方
- 图标画马蹄铁：canter 是马的一种步法，形状也和弯月（kimi）、折角（agy）等分得开。
- 颜色取绿：另外七家分别占了橙、青、白／灰、紫、黄、蓝、粉，只剩绿。
- 两个旧测试原来按“排在最后”断言（amp、agy 是倒数两个；agy、kimi 是倒数两个），加了 canter 后改成按绝对位置 `Tool::ALL[4..6]`、`[5..7]`，检查的顺序不变。
- 远程 New Agent 的名单（`remote_start::KINDS`）没加 canter；远程自定义命令名字恰好是 `canter` 时会带上 canter 的图标，这是现有“名字等于种类就用它的图标”的规则，没改。
- Settings › Instructions 用自己的名单，不随替身多出 canter，没动。

### 没做的事
- 暂停、沙箱、Integrations 的 canter 一行（P5-196）；远程 New Agent 的 canter；Instructions 里 canter 的说明文件。
- 真实 canter 起成 corral agent 的联调：等 ranch 让 corral 认得 canter，主控合并安装后做。

### 踩过的坑
- 一条命令里误写了没有输入的 `cat > 文件`，等标准输入卡住，停掉后重写；只留下一个空文件，随后被正式内容覆盖。
- 本机 ImageMagick 读不了 PNG，拼不了对照图，改为逐张看 `rsvg-convert` 的输出。

## 主控审查

- 看了 diff：只动了任务列的文件；现有七家的命令、顺序、颜色、直通规则没变；远程 New Agent 的名单没加 canter；暂停、沙箱、Integrations 没碰。「不要做」里的一件都没做。
- 重跑：`cargo test --all-targets` 1102 过、0 失败，和 agent 说的一致；clippy `-D warnings`、`fmt --check`、`git diff --check` 通过。
- 取舍表态：两条旧测试从“在最后”改成按固定位置断言，检查的顺序不变，同意。远程自定义命令叫 `canter` 时显示 canter 图标是现有按名字认图标的规则，同意不改。
- 用户 11-03（经 canter/main 转）：远程机器的 New Agent 这次不做选 canter，照主控建议。
- 主控没起窗口看；从分支打包装到 `~/Applications/paddock.app`，图标和颜色留给用户看。真实 canter 起成 corral agent 要等 ranch R56 部署后联调。
- 结果：一次过

## 用户验收

- 11-03 装上在 New Agent 里看过：“图标没问题，先合并吧”。
- 看的时候用户另外发现 Settings › Integrations、Instructions、Dispatch 里都没有 canter，Integrations 里 kimi 显示 Not found；记进新任务 P5-196，不在本件改。
- 真实 canter 起成 corral agent 还没验：等 ranch R56 部署后联调。
