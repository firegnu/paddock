# 任务：集成 agy agent

2026-10-14，paddock/main 记；10-15 paddock/main 交给 paddock/dev-agy-1（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（常规 0.85）、交叉审查要、影响面碰要害（安全隐私 0.81）；推翻：往 `~/.gemini` 写文件的是 corral 已发布的 `integration install`，paddock 只在用户确认后调用，这条路 P5-98 amp 已经走过，没有新的隐私面）
类型：功能变更
依据：用户 10-14：“加一个draft 任务，集成agy agent”。之后 ranch R5 调研、R8「接入 agy」做完部署，ranch/main 回报，用户让它直接转来：“请你接入 agy”，并转达用户原话：“主要是想通过agy加进来读屏幕的能力，agy很少用。”ranch/main 据此说“按 amp 那套做法接上就行，不用为 agy 额外做花样”。10-15 用户定了图标、Prompt 必填的做法和验收（见「用户定的」），同意派出。经过见文末「讨论记录」。
依赖：ranch R8（接入 agy，已部署：corral、ranch 都是 `versions/f86d1c2`，`corral upgrade --all` 已跑）。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- 仓库根的 `CLAUDE.md`（规矩，尤其「与 Saddle 分开」「独立编译目录」「桌面窗口测试」「验证」）。
- `docs/任务/P5-98-集成amp.md`：amp 是怎么接的，agy 照它做；对应代码在 git 里（`git log --oneline --grep P5-98`）。
- `docs/DESIGN.md` 里 P5-94（Integrations）、P5-98（amp）、P5-61（Agent shell）那几条。
- ranch 只读：`/Users/firegnu/Developer/personal_projs/ranch/docs/CONTRACT.md` 里 integration 和 agy 的部分、`/Users/firegnu/Developer/personal_projs/ranch/docs/任务/R8-接入agy.md` 末尾「对 paddock 的影响」。不改 ranch。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-88-agy`，分支 `p5-88-agy`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-88-agy`。
- 动 `app/src/integrations.rs`、`settings_integrations.rs`（Status 列、确认框、说明）、`new_agent.rs` 和 `new_agent_view.rs`（多一种 agy）、`agent_shell.rs`、`kind_icon.rs` 和 `app/assets/kinds/`、`preset.rs`、`card.rs`、`settings.rs`（品牌色）、测试、`docs/DESIGN.md` 和本文件。

## ranch R8 给的（公开约定以 ranch `docs/CONTRACT.md` 为准；对 paddock 的影响在 ranch `docs/任务/R8-接入agy.md` 完成记录）
1. `corral integration status` 多一项 `agy`：`hooks:"global"`；`install.path` 是目录 `~/.gemini/config/plugins/corral`；`install` 多一个 `enabled`（只有 agy 有，`true`／`false`／`null`；`false` 是用户在 agy 里停用了插件）。别家没有这个字段，不能当成 `false`。
2. `corral integration install|uninstall --agent agy`：用法和 amp 一样，先 `--dry-run`，用户确认后带 `--yes`；卸载可能带 `warnings`（目录没删掉）。
3. 技能位置多一个 `gemini`（`~/.gemini/config/skills`）：照 status 里的 skills 列表画；装用 `corral install-skills --target gemini --yes`。
4. `corral start <名字> --prompt <首句> -- agy`：agy **必须带 Prompt**（和 amp 相反）。不带就一直 `starting`，`send` 报 `not_idle`。带了还一直 `starting`：插件没装、被停用，或卡在信任目录／登录对话框（要用户 attach 进去点）。
5. agy 等批准时是 `blocked`，来自 corral 读屏，不另标来源。
6. Settings 要讲清楚：插件只在 corral 启动的 agy 里起作用；用户自己开的 agy 每个事件多跑一次 `sh`、什么都不记；Antigravity IDE 可能也加载它，同样什么都不做；装上后已开着的 agy 要重开才生效。

## 要做的（照 P5-98 amp 的做法，不为 agy 另做花样）

### 一、Settings › Agents › Integrations
1. **agy 一行**：名单照旧跟着 status 走。Status 列和 amp 一样按 `install.status`：`create` → `Install`；`same` → 绿色 `Installed` 加 `Uninstall`；`overwrite` → `Update` 加 `plugin changed`；`foreign` → `Not managed`。多一种：`same` 但 `enabled` 是 `false` → 黄色 `Disabled`，旁边小字 `Run agy plugin enable corral`，仍可 `Uninstall`。`enabled` 是 `null` 或没有这个字段时不显示停用。
2. **确认框**（沿用 amp 的样子，文字按 agy）：
   - Install／Update：标题 `Install the corral plugin for agy?`；正文 `Writes 2 files in ~/.gemini/config/plugins/corral:` 加文件名（照 `--dry-run` 报的列）；下面三句：
     - `It only works in agy started by corral. Your own agy sessions run one extra sh per event and record nothing; the Antigravity IDE may load it too and does nothing there.`
     - `agy already open picks it up after a restart.`
   - Uninstall：标题 `Remove the corral plugin for agy?`；正文 `Deletes ~/.gemini/config/plugins/corral.`；下面一句 `agy started by corral will stay Starting until it is installed again.` 有 `warnings`（目录没删掉）时消息条照原文报出。
3. **卡片下的说明**：现在那句 global 说明只写 amp，改成按家各一句，名单里有该家才显示：agy 那句 `agy needs a plugin, installed once in ~/.gemini/config/plugins/corral. It does nothing outside corral.`
4. **Skills 列**：`gemini` 位置照 status 的列表画、照现在的流程装，不另写死。

### 二、New Agent 多一种 agy
5. 种类排在 amp 后面：标签 `agy`，命令 `agy`，没有模型和强度（和 amp、pi、omp 一样）。预设能存 agy。
6. **Prompt 必填**：选 agy 时 Prompt 框下面一句 `agy needs a first message to start.`；Prompt 为空时 Start 按钮变灰、⌘↩ 不起作用。有字时照常 `corral start … --prompt <首句> -- agy`。
7. **插件提醒**：和 amp 一样选到 agy 时后台读一次 status；插件不是 `same` 时黄字 `The corral plugin for agy isn't installed; agy will stay Starting.`；被停用时 `The corral plugin is disabled in agy; run agy plugin enable corral.`；都带 `Open Settings`，不拦着 Start。

### 三、其他写死名单的地方
8. **图标**：真图标主控已放在本机 `~/.config/paddock/icons/agy.png`（Antigravity 官网的 apple-touch-icon，180px，`SOURCES.txt` 已记），不进仓库；你只让 paddock 认它，照 amp 的做法。仓库 `assets/kinds/agy.svg` 是 paddock 自己画的单色剪影，只在本机没有真图标时用。
9. **品牌色**：主题多一项 `agy`，七套预设各给一个和另外五家分得开的颜色，Settings 的 Colours 页可改。
10. **Agent shell**：替身多一个 `agy`。直接跑真 agy、不经 corral 的：非终端、`--help`／`-h`／`--version`、`-p`／`--print`／`--prompt`（一次性打印），和管理子命令（`agent`、`agents`、`changelog`、`help`、`install`、`mcp`、`models`、`plugin`、`plugins`、`remote-control`、`update`、`mic-serve`），按 `agy --help` 的列表。
11. `docs/DESIGN.md`：P5-94／P5-98 那几条补上 agy、`enabled`、`gemini`；P5-61 Agent shell 的种类加 agy。

## 测试
- 只用合成数据和假 `corral` 脚本；开发测试用隔离 HOME，不往用户真实的 `~/.gemini` 装插件。
- 真实 agy 实测等用户在场（ranch/main 转达），不是你的活，见「怎么算做完」5。

## 怎么算做完
用户 10-15 同意主控建议的五条（“五条都同意”）：
1. Integrations 里 agy 一行：插件各状态（含 `Disabled`）显示对；Install／Update／Uninstall 先确认、确认后成功、重读后状态对；说明讲清楚“只在 corral 启动的 agy 里起作用、IDE 可能加载但什么都不做、装完要重开”。
2. New Agent 选 agy：Prompt 空时不能 Start；带 Prompt 能开起来，侧栏有 agy 的图标和颜色，starting → working → idle；插件没装或被停用时有提醒。
3. Agent shell 里敲 `agy` 经 corral 托管，`agy --version`、`agy models` 直接跑。
4. 现有五家一点不变。
5. 实测（用户在场）：从 Settings 装插件、New Agent 开一个 `paddock/test-agy` 带一句话，看到 idle 和回复；试一次等批准看到 blocked；用完 stop；插件留不留用户定。**这一条由主控合并安装后和用户一起做，你不做。**

你这边的验证只做这些：每块改动一条针对它的测试（假 `corral` 脚本、合成数据，照 P5-98 的测试写法），再按 `CLAUDE.md` 跑一次 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check`。不起窗口截图、不录屏。觉得不够，在回复里说，不要自己加。

## 用户定的（10-15）
1. **真图标**：“你自己找，和codex，claude code走一样的路子。”本机没有 Antigravity 程序包，主控照 amp 的路子从官网取了，已放好（见第 8 条）。
2. **Prompt 必填**：选了主控建议的第 6 条（Start 变灰 + 一句说明）。
3. **验收**：“五条都同意”。

## 不要做
- 不往用户真实的 `~/.gemini`、`~/.config/paddock` 写任何东西；不跑不带 `--dry-run` 的 `corral integration install|uninstall`，要跑就在隔离 HOME 里。
- 不用 corral 开真 agy（会用用户的账号），不对 `corral ls` 里现有的 agent 做 `stop`、`send`、`keys`、attach。
- 不改 ranch、Saddle 的仓库；公开约定不够用时停下来报告，不自己绕。
- 不加 `CLAUDE.md`「语言与依赖」不许的依赖，不升级 `gpui-pre-*`。
- 不起 paddock 窗口、不截图、不用脚本模拟按键或鼠标。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录，一条命令能把它们全杀掉。停自己起的进程用起的时候记下的 PID。
- 遇到 ranch 文档和 corral 实际输出对不上、或要改现有五家的行为才能做下去，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-88-agy` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。

## 讨论记录（草稿阶段）
- 10-14 用户收窄：“Settings 里一键配置 agent”另立 P5-94（已做）；cairn 不在范围内。
- 10-14 主控把 agy 的钩子、全局安装、读屏兜底交给 ranch 调研（R5，ranch `7a4b49b`）：agy 只能全局装在 `~/.gemini/config/plugins/corral/`，等批准没有钩子、要读屏。用户同意 R5 的五条：两个全局位置、先 amp 再 agy、读屏最后单独一件、读屏用 vt100、amp 先不支持 `--prompt`。
- ranch R6（amp）、R7（读屏）、R8（agy）先后做完部署；paddock 这边 amp 是 P5-98。

## 完成记录（10-15，paddock/dev-agy-1）

### 做了什么
- `integrations.rs`：`Install` 多一个 `enabled`（只读 `install.enabled` 的 true／false，`null`、没有这个字段都是没说）；Status 多一种 `Disabled`：`same` 且 `enabled` 为 `false` 时，旁边一句 `Run agy plugin enable corral`，Uninstall 照样给；`plugin` 做成后把 corral 的 `warnings` 带回来（agy 卸载时目录没删掉）；`plugin_status` 给 New Agent 整个 `Install`（状态和 `enabled`）。去掉只给旧说明用的 `global`。
- `settings_integrations.rs`：每家插件的话放在一处（amp 的原样搬过去，agy 照任务文字）：卡片下按家各一句，名单里有该家才显示；确认框 agy 写 `Writes 2 files in ~/.gemini/config/plugins/corral:` 列 `hooks.json`、`plugin.json`（Update 时标 replace），卸载写 `Deletes ~/.gemini/config/plugins/corral.` 和 Starting 那句；消息条成功后接上 corral 的 warnings 原文。`Disabled` 是黄色圆角标加安静的 Uninstall，那句小字放在下一行（Status 列 190 宽，放旁边挤不下）。Skills 列没改：gemini 位置照 status 的列表画、照原来的流程装。
- `new_agent.rs`：`Tool::Agy`（标签、命令 `agy`，没有模型和强度，排在 amp 后，预设能存）；`needs_prompt`、`has_plugin`、`Form::can_start`（agy 的 Prompt 去掉空白后为空时不能 Start）；`plugin_warning`：没装、旧了、被停用三种黄字。agy 带 Prompt 照常 `--prompt <首句> -- agy`。
- `new_agent_view.rs`：选 agy 时 Prompt 下一句 `agy needs a first message to start.`；不能 Start 时按钮变淡、不响应，⌘↩ 也直接返回；插件状态按种类读（记着问的是哪一家，amp、agy 来回切会重读，晚到的旧回答不用）。
- 图标：`kind_icon.rs` 种类加 `agy`，认本机 `~/.config/paddock/icons/agy.png`；仓库 `assets/kinds/agy.svg` 是自己画的剪影（离地的折角），README 写明不是官方标志。
- 颜色：主题加 `agy`，七套各一个蓝（和橙、青、白、紫、黄分开）；Settings 颜色页 Agent types 一组多一项；卡片品牌色认 agy。
- Agent shell：替身加 `agy`；`--prompt` 和 `agy --help` 列的十二个子命令直接跑真程序，`-i`／`--prompt-interactive` 照样托管。
- `docs/DESIGN.md`：P5-61 加 agy，P5-94 补 gemini 位置和 agy，P5-98 那句说明改成按家，§13 加 P5-88 一条。

### 验证了什么
- 每块一条针对它的测试，合成 JSON 和假 corral 脚本：integrations 4 条（`enabled` 读法、`Disabled` 和其余状态及 gemini 技能、卸载带回 warnings、New Agent 读到停用）；settings_integrations 3 条（说明按家、确认框的文件、消息条接 warnings）；new_agent 3 条（agy 必填和 start 参数、预设存 agy、三种提醒和 amp 不变）；kind_icon、preset（六家颜色两两不同）、card（agy 品牌色）、agent_shell 各 1 条。kind_icon、preset、card、agent_shell 的先跑看到断言失败再实现；integrations、settings_integrations、new_agent 的新测试先跑是编译不过（新字段和函数还没有），不算严格的 RED。
- `app/` 下 `cargo test --all-targets`（库内 533 条和各集成测试全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。settings 颜色总数的断言 42 改 43（多了 agy）。
- 只在隔离 HOME（scratchpad）跑过 `corral integration install|uninstall --agent agy --dry-run` 和 `integration status`，核对输出格式；没装、没卸真的插件，没开 agy，没起窗口、没截图。

### 拿主意的地方
- `--dry-run` 只报目录 `path`，不列文件（任务写“照 `--dry-run` 报的列”）。文件名照 ranch CONTRACT（“里面只有 corral 的 `hooks.json`、`plugin.json`”）写在 paddock 里；CONTRACT 改了这里要跟着改。
- New Agent 里插件 `overwrite` 时和 amp 一样说 `…has changed; update it in Settings.`，不说“没装”；停用只在 `same` 时判断，和 Settings 一致。
- Prompt 只有空白也算空。
- amp 装／更新成功时如果 corral 带了 warnings，现在也会接在消息条后面（以前丢掉）；amp 成功时 corral 实际不带 warnings，界面看起来不变。

### 没做的
- paddock 自己的技能（`skills.rs`）只装 claude、codex 两个位置，没有 gemini：agy 那一行 Skills 只算 corral 和 corral-dispatch，agy 读不到 paddock 技能。任务说照原流程、不另写死，`skills.rs` 也不在要动的文件里，没改。
- New Agent 底下的提示 `empty task starts it idle` 对 agy 不对（agy 空着开不了），没改。
- 真实 agy 实测（「怎么算做完」5），留给主控和用户。

## 主控审查（10-15，paddock/main）
- 看了全部 diff（13 个文件）：只动了任务列的文件；「不要做」一件没碰（插件装卸只在隔离 HOME 跑 `--dry-run`，没开真 agy，没起窗口）。现有五家：amp 的文字原样搬进 `PluginWords`，状态、确认框、提醒行为不变；只多了“成功时把 corral 的 warnings 接在消息条后面”，amp 成功时不带 warnings，看起来不变，同意。
- 「怎么算做完」1–4 在代码和测试里都对得上；第 5 条（真实实测）由主控合并安装后和用户一起做。
- 重跑一次 `cargo test --all-targets`：全过，库内 533 条（10-15 更正：原写“合计 633、和回复的 653 对不上”，是主控只看了输出前 20 行、漏数；653 是对的）。
- 取舍：文件名照 CONTRACT 写死（同意，CONTRACT 改了要跟着改）；`overwrite` 说“has changed”（同意）；只有空白算空（同意）。
- 建议改（不挡合并，交用户定）：New Agent 底部 `empty task starts it idle` 对 agy 不对；paddock 技能没装到 gemini 位置，agy 读不到 paddock 技能。

## 实测（10-15，用户在场，验收第 5 条）
- 用户重开 paddock，从 Settings › Integrations 装了 agy 插件；`corral integration status`：agy 1.3.5，install `same`、`enabled: true`，gemini 技能 `same`。
- 主控开 `paddock/test-agy`（`--label role=test`、`--prompt` 一句、`-- agy`，临时目录）：→ idle，回复「你好，我是 agy。」。
- 要它跑 `echo hi > hello.txt`：working → blocked（批准框没点，文件没建）。第一次送时用户正开着它的窗格，`corral send` 回 `human_active`，用户退出后再送成功。
- 用户在侧栏看过：“看过了，停掉吧，插件留着”。主控已 `corral stop paddock/test-agy`；插件留在 `~/.gemini/config/plugins/corral`。
