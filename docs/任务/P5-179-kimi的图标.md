# 任务：kimi 的图标，New Agent 能选 kimi

2026-10-24，paddock/main 交给 paddock/dev-kimi（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.68）、拿不准、拿不准；交叉审查主控定不要：往 kimi 配置写的是 corral 已发布的 integration install，paddock 只在用户确认后调用，P5-98、P5-88 走过同一条路）
类型：功能变更
依据：用户 10-24 要 kimi 的图标、New Agent 能选 kimi，并答了主控问的四处顺带的地方（见「用户原话」）。照 P5-88 接 agy 的做法接 kimi，不另做花样。ranch R48c 已让 corral 支持 kimi（本机 corral 已能报 kimi，钩子已装好）。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- “同时你也新建一个draft task：kimi的图标”
- “我理解应该和claude code/codex一致的方法，我记得是官方的留在本地。”
- “顺带 New Agent 也能选 kimi”
- 主控问的四处（10-24，用户逐条选）：Settings › Integrations 给 kimi 一行、New Agent 钩子没装时提醒 → “要”；Agent shell 里敲 `kimi` 经 corral 起 → “要”；New Agent 选 kimi 时模型 → “不选，用 kimi 默认”；远程机器的 New Agent 也能选 kimi → “要”。

## 先读
- 仓库根的 `CLAUDE.md`（规矩，尤其「独立编译目录」「桌面窗口测试」「验证」）。
- `docs/任务/P5-88-集成agy.md`：agy 是怎么接的，kimi 照它做；对应代码在 git 里（`git log --oneline --grep P5-88`）。
- `docs/DESIGN.md` 里 P5-61（Agent shell）、P5-94（Integrations）、P5-98（amp）、P5-88（agy）、P5-90／P5-122 远程新建那几条。
- ranch 只读：`/Users/firegnu/Developer/personal_projs/ranch/docs/CONTRACT.md` 的 integration 部分和「Kimi Code 阶段 1（R48c）」一节。不改 ranch。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-179-kimi`，分支 `p5-179-kimi`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-179-kimi`（已从 `.target/main` 克隆）。命令在 `app/` 下跑。
- 会动的大致是：`kind_icon.rs` 和 `app/assets/kinds/`（含 README）、`preset.rs`、`card.rs`、`settings.rs`（品牌色）、`new_agent.rs`、`new_agent_view.rs`、`integrations.rs`、`settings_integrations.rs`、`agent_shell.rs`、`remote_start.rs`、测试、`docs/DESIGN.md` 和本文件。以实际需要为准。

## ranch R48c 给的（以 ranch `docs/CONTRACT.md` 为准）
1. `corral integration status` 多一项 `kimi`：`hooks:"global"`；`install.path` 是 Kimi 的共享配置文件（`$KIMI_CODE_HOME/config.toml`，没设就 `~/.kimi-code/config.toml`），`install.format:"marked-block"`，没有 `enabled`。技能位置沿用 `codex`（`~/.agents/skills`），不新增位置。
2. `corral integration install|uninstall --agent kimi`：用法和 amp、agy 一样（先 `--dry-run`，确认后 `--yes`）。只加、删 corral 那一段标记块，块外不动；卸载保留配置文件本身。
3. `corral start … -- kimi`：`--prompt` 可选（不像 agy 必填）。等批准、提问时是 `blocked`。

## 要做的

### 一、图标（和 claude、codex 一样）
1. 仓库 `app/assets/kinds/kimi.svg`：paddock 自己画的单色剪影，不是官方标志；README 记为 “Original drawing for paddock, not the official mark.”，并在 README 认的名字里加 `kimi`。
2. 本机有 `~/.config/paddock/icons/kimi.png`（或 `.svg`）时画它：主控已放好官方原图（Kimi 官网的 PWA 图标，512px，`SOURCES.txt` 已记），不进仓库；你只让 paddock 认 `kimi` 这个名字。
3. 主题多一项 `kimi` 品牌色：每套预设各给一个和另外六家分得开的颜色，Settings 的 Colours 页可改。

### 二、New Agent
4. 种类多一个 kimi，排在 agy 后：标签 `kimi`，命令 `kimi`，没有模型和强度（用户选“不选，用 kimi 默认”），预设能存。Prompt 可选，行为和 claude、codex 一样（空 Prompt 照常起成 idle）。
5. 钩子提醒：和 amp、agy 一样，选到 kimi 时读一次 status；没装、旧了等不是 `same` 时黄字提醒加 `Open Settings`，不拦着 Start。文字按 kimi 实际后果写（读 CONTRACT 确认没装钩子时 corral 开的 kimi 会怎样，不要照抄 agy 的 “stay Starting”，除非确实如此）。

### 三、Settings › Agents › Integrations
6. kimi 一行：Status 列照 amp 的状态规则（`create`／`same`／`overwrite`／`foreign`）；没有 `enabled` 就不显示停用。
7. 确认框按 marked-block 写：装／更新说明只在那个共享配置文件里加 corral 的一段、其余内容不动、只在 corral 启动的 kimi 里起作用、开着的 kimi 要重开；卸载说明只删 corral 那一段、配置文件保留。路径照 status 报的 `install.path` 显示（`~` 缩写照现有做法）。认 `install.format`，不按 kind 名猜装法。
8. 卡片下的说明按家各一句，名单里有 kimi 才显示。

### 四、Agent shell
9. 替身多一个 `kimi`。直接跑真 kimi、不经 corral 的：非终端、通用的 `--help`／`-h`／`--version`／`-V`、`-p`／`--prompt`（一次性打印），和 `kimi --help` 列的管理子命令（现在是 `export`、`fork`、`provider`、`session`、`acp`、`web`，以你跑出来的为准）。

### 五、远程新建
10. 远程机器的 New Agent（`remote_start.rs`）也能选 kimi：远端找得到 `kimi` 时列出来，排在 omp 后。远端钩子装没装不在这次查（简单对话框没有这个位置）；如果你发现不装钩子 kimi 根本起不来，停下来问主控。

### 六、文档
11. `docs/DESIGN.md`：P5-61、P5-94、远程新建那几条补上 kimi；加一条「集成 kimi（P5-179）」，写用户原话和定下的做法。

## 怎么算做完
用户的验收原话是“kimi的图标”“顺带 New Agent 也能选 kimi”，加上上面四处用户选的“要”。逐条：
1. 侧栏、标签上 kimi 有图标和颜色：本机有官方图时画官方图，没有时画仓库剪影。
2. New Agent 能选 kimi 并起成 corral agent；钩子没装时有提醒。
3. Integrations 里 kimi 一行：状态显示对，装／更新／卸载先确认、确认后成功、重读后状态对，说明讲清楚只改共享文件里的一段。
4. Agent shell 里敲 `kimi` 经 corral 托管，`kimi --version`、`kimi session …` 直接跑。
5. 远程 New Agent 远端有 kimi 时能选。
6. 现有六家一点不变。
- 验证只做这些：给新逻辑加针对性单元测试（种类列表、图标名、status 解析 marked-block、确认框文字、替身直通、远程找到 kimi）；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。觉得不够，在回复里说，不要自己加。
- 真实 kimi 实测（开一个 `paddock/test-kimi` 看 idle、回复、blocked）由主控合并安装后和用户一起做，你不做。

## 不要做
- 只用合成数据和假 `corral` 脚本测试；开发测试用隔离 HOME，不往用户真实的 `~/.kimi-code/config.toml` 装、卸钩子，不运行真的 `corral integration install|uninstall`。
- 不开真实 kimi 或任何 corral agent，不碰 `corral ls` 里已有的 agent（不 stop／send／keys）。
- 不读 Kimi 的内部目录（`~/.kimi-code/` 下除了 status 报的配置路径以外的东西），不读凭证、token。
- 不改 ranch、Saddle 仓库；不把 `~/.config/paddock/icons/` 里的官方图抄进仓库。
- 不加 Settings › Instructions 里 kimi 的说明文件（用户没要）。
- 不按名字批量杀进程（`pkill -f paddock` 这类）；停自己起的进程用启动时记下的 PID。
- 截图、起测试窗口照 CLAUDE.md「桌面窗口测试」；不用脚本模拟按键或鼠标。
- 碰到要拿主意的事（包括第 5、10 条说的情况），停下来，在回复里写清问题、可选办法和你的建议，然后结束这一轮，等主控回复再接着做；不在自己窗口里问用户，不开着命令等。主控的决定写进完成记录。
- 不合并到 main，不推送。只在 `p5-179-kimi` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 完成记录

前一个 agent（paddock/dev-kimi-1）做完第一节后机器重启，留下没提交的改动。我（paddock/dev-kimi 接手）看过 diff：kimi.svg、README、kind_icon.rs（认 `kimi`、剪影、本机原图测试）、card.rs、preset.rs（各套主题的 kimi 玫红／粉）、settings.rs（Colours 分组和测试）都对，原样接着用，从第二节做起。

### 做了什么
- New Agent：`Tool::Kimi` 排在 agy 后，标签、命令 `kimi`，没有模型和强度，预设能存；Prompt 可选，同 claude、codex（空着起成 idle，底部照常 `empty task starts it idle`）。选到 kimi 时读一次 status，不是 `same` 时黄字加 `Open Settings`：`corral's hooks for kimi aren't installed; corral won't see kimi working or replying.`，改过时 `corral's hooks for kimi have changed; update them in Settings.`。
- Integrations：`Install` 读 `install.format`，`marked-block` 才算一段（`Install::block()`，不按种类名猜，不认识的 format 照文件／目录处理），`PluginPlan` 带 `block`。Status 照 amp 的规则，没有 Disabled；一段时的小字是 `corral block changed`、`corral block in config.toml isn't corral's`。确认框：标题 `Install／Update／Remove corral's hooks for kimi?`；装／更新 `Adds corral's marked block to this shared file; nothing else in it changes:`，列出路径（`~` 缩写），标 `add block`／黄字 `replace block`，再说只在 corral 启动的 kimi 里起作用、开着的 kimi 要重开（`picks them up after a restart`）；卸载 `Removes only corral's block from <路径>; the file and everything else in it stay.`，再说卸了以后 corral 还认得出就绪、看不到每一轮和回复。消息条 `Installed corral's hooks for kimi` 等。卡片下 kimi 一句，名单里有 kimi 才显示。amp、agy 的文字一字没变。
- Agent shell：替身多 `kimi`；`-V`、`--prompt` 和 `kimi --help`（2.3.1）列的 17 个子命令直接跑真程序。
- 远程 New Agent：`remote_start::KINDS` 加 kimi，排在 omp 后；一种都没找到时那句改成 `No Claude, Codex, pi, omp or kimi found on <机器>.`。
- DESIGN：P5-61、P5-94、P5-124 补 kimi，§13 加「集成 kimi（P5-179）」。

### 验证了什么
- 新加的单元测试：种类列表和顺序、表单参数（有无首句）、预设存读、钩子提醒文字（new_agent）；status 解析 marked-block、Status 列和小字、plan 带 block、install／uninstall 调 `--agent kimi`（integrations，假 corral 脚本）；卡片说明只在名单有 kimi 时出、确认框和卸载文字、按 plan 而不是名字认一段（settings_integrations）；替身直通和托管（agent_shell）；远程找得到 kimi 且排在 omp 后（remote_start）。agent_shell、remote_start 两处先跑到 RED 再实现；New Agent、Integrations 的测试和新字段、新种类一起写，之前没法编译，没有单独看到 RED。
- `app/` 下 `cargo test --all-targets`（1044 个全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。
- 没开窗口看界面：确认框、提醒条的排版没有亲眼看过，留给主控和用户实测时看。

### 拿主意的地方
- 提醒文字：读了 ranch CONTRACT 和 ranch DESIGN 的 R48c：kimi 的编辑器就绪由 corral 读屏认出，钩子没装也会显示就绪；之后的 working、blocked、完成、回复和带首句时的状态都靠钩子。所以没照抄 agy 的 “stay Starting”，写成 corral 看不到它干活和回复。
- 第 10 条：不装钩子 kimi 也起得来（就绪靠读屏），没有停下问主控，照做了。
- Agent shell 子命令：任务里写的是 6 个，`kimi --help` 实际列了 17 个（多了 `server`、`rc`／`remote`、`login`、`doctor`、`vis`、`install-desktop`、`app`、`migrate`、`upgrade`／`update`），按跑出来的全列；`fork` 只是非交互地复制会话（看过 `kimi fork --help`），也直通。kimi 的版本参数是 `-V`，补在 kimi 自己的参数里。
- `instructions::KINDS` 原来直接借 `agent_shell::KINDS`，加了 kimi 后 Settings › Instructions 的名单也会多 kimi（`tests/instructions.rs` 因此失败）。任务说不加 kimi 的说明文件，所以让 Instructions 用自己写明的六家名单，测试原样不动，行为不变。
- 确认框文件旁的标记从 bool 改成文字（`new`、`replace`、`add block`、`replace block`），`replace` 开头的黄字，技能确认框照旧只出 `new`／`replace`。
- 品牌色沿用前一个 agent 定的值（暗色多取各主题自带的粉／红，浅色取暗一些的玫红），`every_theme_tells_the_agent_kinds_apart` 过了。

### 没做的事
- 真实 kimi 实测（idle、回复、blocked）：照任务留给主控合并安装后和用户做。
- 远端钩子装没装：照任务不查。
- Settings › Instructions 里 kimi 的说明文件：用户没要。

### 踩过的坑
- 用 perl 的 `s{…}{…}` 批量改 Rust 代码时，替换文本里的 `{kind}`、`{}` 和分隔符打架，几次报错没改成；改用 Edit 工具或 `#` 分隔符。
- `instructions::KINDS` 和 Agent shell 共用一份名单，事先没想到，全量测试才发现（见上）。
