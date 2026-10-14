# 任务：集成 amp agent

2026-10-14，paddock/main 记。
依据：用户 10-14：“再加一个draft task，我还有amp需要集成”。之后 R5 调研的五条用户都同意（“按你的建议，五条都同意”），其中「amp 先不支持 `--prompt`，New Agent 选 amp 时不给 Prompt」。ranch R6 做完、部署后，ranch/main 转达：“用户要你接着把 paddock 这边的集成做完，按你自己的任务流程来。”经过见文末「讨论记录」。
依赖：ranch R6（接入 amp，ranch `c376416`／收尾 `0e5bd19`，10-14 19:01 已部署）。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-98-amp`，分支 `p5-98-amp`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-98-amp`。
- 动 `app/src/integrations.rs`、`app/src/settings_integrations.rs`（Status 列、确认框、说明）、`app/src/new_agent.rs` 和 `new_agent_view.rs`（多一种 amp）、`app/src/agent_shell.rs`（替身多一个 `amp`）、`app/src/kind_icon.rs` 和 `app/assets/kinds/`（图标）、`app/src/preset.rs`、`card.rs`、`settings.rs`（品牌色）、测试、`docs/DESIGN.md` 和本文件。

## ranch R6 给的（公开约定 ranch `docs/CONTRACT.md`，主控 10-14 核对过本机输出）
- `corral integration status` 多第五项 `amp`：`hooks:"global"`，多一个 `install:{path,status,version}`；`status` 是 `create`／`same`／`overwrite`／`foreign`，约定写明前端分别显示 Install／Uninstall／Update／只显示说明。amp 的技能在 claude、codex 两个位置。本机现在是 `same`（插件已装，`~/.config/amp/plugins/corral.ts`，版本 1）。
- `corral integration install|uninstall --agent amp [--dry-run] [--yes]`：只写／删那一个文件；前端调必须带 `--yes`（先让用户确认），否则报 `confirmation_required`；`foreign` 不动；确认后文件变了报 `changed`、什么都不写。做完再读一次 status。
- `corral start … -- amp` 不能带 `--prompt`（报 usage）；要第一句就 start 后 `corral wait` 到 idle 再 send。一直 `starting` 多半是插件没装、或 amp 是装插件之前开的。
- amp 等批准报 `blocked`，state 取值没有新增；旧版 corral 开的 amp 要重开。现有四家不变。

## 样稿
`docs/设计稿/P5-98-amp/`（原稿在 claude.ai 私有画布「amp集成样稿」）：1 Agents 页 amp 一行、2 四种状态、3 Install 确认框、4 Uninstall 确认框、6 New Agent 选 amp（B，采用；5 是没采用的 A）。

## 要做的

### 一、Settings › Agents › Integrations
1. **amp 那一行**（名单照旧跟着 status 走，不写死）：
   - Agent 列：`amp`，下面是版本。amp 的版本行是 `0.0.1791964847-g13f24a (released 2026-10-14T08:00:47.000Z, 3h ago)`，括号里的“3h ago”是读的那一刻算的，放着会过时：从 ` (released ` 起去掉，只显示前面的版本号。
   - Status 列按 `install.status`：
     - `create`：`Install` 按钮（和 Skills 的按钮同一个样子）。
     - `same`：绿色 `Installed`（和 `Built in` 同一种圆角标），旁边一个安静的 `Uninstall`（文字按钮，悬停才显出底）。
     - `overwrite`：`Update` 按钮，旁边小字 `plugin changed`。
     - `foreign`：灰字 `Not managed`，旁边小字 `corral.ts isn't corral's`；没有按钮。
     - 不认识的值照原样显示，不报错。
   - 没找到 amp（`path` 为空）：和别家一样整行变淡、`—`、没有按钮。
2. **确认框**（沿用 P5-94 的确认框样子，英文）：
   - Install／Update：标题 `Install the corral plugin for amp?`（Update 时 `Update …`）；正文 `Writes 1 file:` 加路径 `~/.config/amp/plugins/corral.ts`（Update 时标 `replace`）；下面三句：
     - `It only works in amp started by corral. Your own amp sessions and the Amp desktop app may load it too, but it does nothing there.`
     - `Each amp that loads it runs one more idle plugin process.`
     - `amp already open picks it up after a restart.`
   - Uninstall：标题 `Remove the corral plugin for amp?`；正文 `Deletes ~/.config/amp/plugins/corral.ts.`；下面一句 `amp started by corral will stay Starting until it is installed again; amp already open keeps it until restarted.`
   - 确认后在后台跑 `corral integration install|uninstall --agent amp --yes`；底部消息条报 `Installed the amp plugin`／`Updated …`／`Removed …`，失败红字带原因（`changed` 时写 `The file changed while you were deciding; nothing was written.`）。做完重读。
3. **卡片下面的说明**：第一句现在是 “Status comes with corral each time it starts an agent; there is nothing to install.”，对 amp 不对了。改成两句：`Built in: status comes with corral each time it starts an agent.` 和（名单里有 `global` 的 agent 时才显示）`amp needs a plugin, installed once in ~/.config/amp/plugins. It does nothing outside corral, but each amp that loads it, the desktop app too, runs one idle plugin process.` Skills 那句不变。
4. Skills 列不用改：amp 读的 claude、codex 两个位置 P5-94 已经会装。

### 二、New Agent 多一种 amp
5. 种类排在 omp 后面：标签 `amp`，命令 `amp`，没有模型和 effort（和 pi、omp 一样）。预设能存 amp。
6. **Prompt（用户选 B）**：选 amp（命令的程序是 `amp`）时 Prompt 框照常能填，下面一句 `amp starts first; paddock sends this once it is idle.`。start 时不传 `--prompt`；start 成功后照常打开 agent 窗格、关掉面板，后台 `corral wait <名字> --timeout 60` 等到 `idle` 再 `corral send <名字> <原文>`。送不出去（等到的是 blocked、一直 starting、send 被拒）时侧栏底部红字说明原因，原文放进剪贴板，不丢；送成功不另报。Prompt 为空时只 start。
7. **插件没装的提醒**：选 amp 时在后台读一次 `corral integration status`（只在选到 amp 时读，一次一两秒）；`install.status` 不是 `same` 时种类下面一句黄字 `The corral plugin for amp isn't installed; amp will stay Starting.` 加一个 `Open Settings` 按钮（打开 Settings › Agents）。不拦着 Start。

### 三、其他写死名单的地方
8. **图标**：用 Amp 真的图标（用户 10-14）：`~/.config/paddock/icons/amp.png`，取自 Amp 官网 `https://ampcode.com/app-icon.png`（512 px，主控 10-14 放好，SOURCES.txt 记了来源），和另外四家的真图标一样只放本机、不进仓库（Amp 的标志不是开源许可）；`kind_icon.rs` 的种类加 `amp`。仓库里 `assets/kinds/amp.svg` 只是本机没有真图标时的退路：paddock 自己画的单色剪影，README 写明不是官方标志。
9. **品牌色**：主题多一项 `amp`，七套预设各给一个颜色，Settings 的 Colours 页 Agent types 一组多一项，用户能改。
10. **Agent shell**：替身多一个 `amp`。放行（直接跑真 amp、不经 corral）的：非终端、`--help`／`-h`／`--version`／`-V`／`-v`、`-x`／`--execute`、`--no-tui`，以及 amp 的管理子命令（`login`、`logout`、`account`、`threads`（`threads continue` 除外，它是交互的，照样托管）、`plugins`、`mcp`、`config`、`update` 等，按 `amp --help` 的列表）。
11. `docs/DESIGN.md`：P5-94 那条补上 global 一档；P5-61 Agent shell 的种类加 amp；§3 记上 `corral integration install|uninstall`。

## 怎么算做完
- 用户没另说验收。主控建议下面几条，请用户定：
  1. Integrations 里 amp 一行：插件四种状态各显示对，Install／Update／Uninstall 点了先确认、确认后装／卸成功、重读后状态对；确认框和卡片下的说明讲清楚“装在哪、只在 corral 启动的 amp 里起作用、桌面版也可能加载但什么都不做、每个多一个闲置进程、装完重开已开着的 amp”。
  2. New Agent 选 amp：Prompt 能填、有说明，amp 空闲后送进去；插件没装时有提醒；能开起来，侧栏有 amp 的图标和颜色，状态从 starting 变 idle。
  3. Agent shell 里敲 `amp` 经 corral 托管，`amp --version`、`amp threads list` 直接跑。
  4. 现有四家一点不变。
- 自查：`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过。读 status、算 amp 那一行、该跑什么命令、New Agent 的参数、Agent shell 放行用合成 JSON 和假 corral 脚本测（先写测试看它失败再实现）；不在测试里装、卸真的插件。
- 装上给用户看；主控不起测试窗口、不截图。真实 amp 开一个 `paddock/test-amp`（`--label role=test`）发一句短话看状态，用完 `corral stop`——这一步用用户的 amp 账号，做之前问用户。点 Uninstall／Install 是真改用户的 `~/.config/amp/plugins/`，留给用户自己点。

## 不做
- agy（P5-88）、读屏兜底（ranch 阶段二）、cairn。
- amp 的 `--mode`（low／medium／high／ultra）不在 New Agent 里给选项，和 pi、omp 一样照 amp 自己的设置；要的话另立。
- 不碰桌面版 Amp（`/Applications/Amp.app`），不读 `~/.local/share/amp/`、`~/.config/amp/settings.json`。
- 不改 ranch，只调公开命令。

## 主控拿的主意（用户 10-14 同意）
1. Status 列 `same` 时同时显示 `Installed` 和安静的 `Uninstall`（约定写的是“显示 Uninstall”；只放一个 Uninstall 按钮看不出现在装没装）。
2. 版本号去掉 `(released …, 3h ago)`。
3. New Agent 选 amp 时读一次 status 提醒插件没装（约定说一直 starting 要提示去 Settings 装；主控选在开之前提醒，不在侧栏卡片上等它卡住再提示）。
4. 图标用 paddock 自己画的剪影（用户改：用真的，见第 8 条），品牌色主控在七套主题里各挑一个（和 Claude 的橙、Codex 的青、pi 的白、omp 的紫分开），用户可改。
5. 先出样稿（Integrations 的 amp 四种状态、两个确认框、New Agent 选 amp）再动手，还是照上面的文字直接做：主控建议先出样稿，主要是确认框和说明的文字、Uninstall 的样子。

用户 10-14：“接受你上面的建议，但是AMP的图标，你最好用真的。因为现在claude和codex以及omp还有pi都是真的”。五条照建议定（先出样稿），图标改用真的。

看过样稿（`docs/设计稿/P5-98-amp/`，画布「amp集成样稿」）。主控指出 New Agent 的中心就是 Prompt 大框，提 A（照原来不给 Prompt，框变灰）和 B（照常填，paddock 等 amp 空闲后送）两种，建议 B。用户 10-14：“选B，样稿可以，开始吧”。

## 讨论记录（10-14，由旧到新）

### 草稿时的现状（主控 10-14 查的，只读）
- 本机装了 `amp`：`/opt/homebrew/bin/amp`。
- 当时 corral 只认 claude、codex、pi、omp 四家；herdr 内置的 18 家里也没有 amp。
- 原 Python 版 corral 调查过（`../corral/docs/ROADMAP.md` R5，2026-09-17）：事件够用；插件只能从设置、项目 `.amp/plugins/`、全局 `~/.config/amp/plugins/` 加载，当时 corral 不动全局配置所以没接；第一句话带不进去；`tool.call` 要表态。
- 用户 10-14 已同意“全局配置可以放宽”（经用户确认才装、不在 corral 启动的 agent 里什么都不做、只动自己那块、原子写、能卸干净）。

### 交给 ranch 调研（R5）
- 用户 10-14：“我的理解是这样的。读屏幕我也倾向于做一个兜底的策略。你直接发给ranch让他调研吧”。主控 `corral send` 给 `ranch/main`（request_id d5753c97-ef35-404f-9cdb-a3ff02335ada）。
- ranch 回报 R5（ranch `7a4b49b`，`docs/调研/R5-agy与amp接入、全局安装和读屏兜底.md`）：agy、amp 都只能全局安装；amp 能报全（缺首句），每个 amp 多一个闲置 Bun 进程；读屏兜底放 pen 里用 vt100；status 加 `hooks:"global"` 和 `install`；新命令 install／uninstall。
- 用户 10-14：“按你的建议，五条都同意，发给ranch吧，但是发给他先不让他动作，我还要和他讨论。”五条：接受两个全局位置；真实测试用临时 HOME 测试实例（先查 agy 登录是否在钥匙串）；先 amp，再 agy，读屏最后；读屏用 vt100；amp 先不支持 `--prompt`。主控 `corral send` 给 `ranch/main`（request_id 8293c134-c333-4a2c-8048-8a910afe1144），写明只记下、不动手。

### ranch R6（用户直接和 ranch 定的，见 ranch `docs/任务/R6-接入amp.md`）
- 用户对 ranch：“amp我已经登录了，注意下是amp cli。另外还有一个app是桌面版本的amp，不要动。”；桌面版也可能加载插件：“我觉得我可以接受。但是接入的时候要在settings里面讲清楚，不过这是paddock界面的事情，暂时你不用关心。”；阶段改成 amp → 读屏兜底 → agy；等批准看终端标题（B）；“amp要支持corral的一些派遣，托管，之间对话等等功能。”
- 10-14 19:01 部署，ranch/main 转达（见依据）。

## 完成记录（10-14，主控）

### 做了什么
- `integrations.rs`：读 status 里的 `install{path,status}`；Status 列换成枚举（`Built in`、`Install`、`Installed`、`Update`、`Not managed`、照原样的词），`Update`／`Not managed` 旁有一句说明；版本号从 ` (released ` 起去掉；`plugin_plan` 只在这一行给按钮时给计划；`plugin` 先 `--dry-run` 看文件还是确认时的样子（变了报 `the file changed while you were deciding; nothing was written`、什么都不写），再 `--yes`，`written` 不是 true 时带 corral 的 warnings 报错；`plugin_status` 给 New Agent 读一家插件的状态。
- `settings_integrations.rs`：Status 列按样稿画（绿色 `Installed` 加安静的 `Uninstall`，`Install`／`Update` 和 Skills 同一种按钮）；确认框分技能和插件两种，插件的装／更新、卸载照样稿文字；标题旁写 `Installing…`／`Removing…`；消息条报 `Installed the amp plugin`／`Updated …`／`Removed …`，失败带原因；卡片下三句说明，amp 那句只在有 `global` 的 agent 时出现。Status 列加宽到 190、Skills 210，窄于 600 时两列挪到名字下面。
- `new_agent.rs`：`Tool::Amp`（命令 `amp`，没有模型和强度，排在 omp 后）；`takes_prompt`；amp 时 start 不带 `--prompt`，`first_message` 留着；`send_first`：`corral wait <名字> --timeout 60`，`idle` 才 `send`，一直 starting、blocked、别的状态、send 被拒都报原因。
- `new_agent_view.rs`：选 amp 时 Prompt 下一句 `amp starts first; paddock sends this once it is idle.`；后台读一次插件状态，没装（或不是 corral 的）黄字 `The corral plugin for amp isn't installed; amp will stay Starting.`，旧了写 `…has changed; update it in Settings.`，带 `Open Settings`（打开 Settings › Agents）；面板重开时重读。`window.rs`：Started 带上第一句，开好窗格后后台送；送不出去侧栏底部红字说原因，原文放进剪贴板。
- 图标：`~/.config/paddock/icons/amp.png`（ampcode.com 的 app-icon，本机，不进仓库，SOURCES.txt 记了）；`kind_icon.rs` 种类加 `amp`；仓库 `assets/kinds/amp.svg` 是自己画的三根电平条，README 写明。
- 颜色：主题加 `amp`，七套各一个暖黄；Settings › Appearance 的 Agent types 一组多一项；卡片品牌色认 amp。
- Agent shell：替身加 `amp`；`-x`／`--execute`、`--no-tui`、`-V` 和 `amp --help` 列的管理子命令直接跑真程序，`threads continue` 照样托管。
- `docs/DESIGN.md`：§3 那行加 install／uninstall，P5-61 加 amp，§13 加 P5-98。

### 验证了什么
- 先写测试看它失败再实现：integrations 8 条（amp 的 install 字段、四种状态和说明、版本号、没找到 amp、计划、装／卸调的命令、确认后变了不写、没写成报原因、New Agent 读插件状态）；new_agent 3 条（amp 不带 `--prompt`、第一句留着、用路径写的 amp 也认；`wait` 到 idle 再 `send`；starting／blocked／别的状态和 send 被拒都不送、报原因）；kind_icon 1 条；preset 1 条（七套主题五家颜色两两不同）；agent_shell 1 条。都用合成 JSON 和假 corral 脚本，没碰真的插件。
- `app/` 下 `cargo test --all-targets`（库内 504 条和各集成测试全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。settings 里颜色总数的断言从 41 改成 42（多了 amp）。
- 没起测试窗口、没截图；没开真的 amp 实测（要用用户的 amp 账号，按任务文件要先问）。样子、点 Install／Uninstall、New Agent 开 amp 送第一句留给用户试。

### 拿主意的地方
- 装／卸前先空跑一次核对状态：corral 带 `--yes` 时不再核对，确认框开着时文件可能变了。
- 插件提醒分两种文字：旧了（`overwrite`）不说“没装”。
- 第一句送不出去时放进剪贴板，免得用户重打；送成功不另报（窗格里看得见）。
- 等 idle 最多 60 秒（corral 技能里写的也是 60）。

### 没做的
- `paddock ctl open --prompt … -- amp`（给 agent 用的 ctl）没改：corral 会直接报 `amp takes no --prompt`，调用的 agent 可以照 corral 技能先 start、wait、再 send。要 paddock 替它送，另立。
- paddock 技能（`skills.rs` 里的 SKILL.md）没提 amp 不收 `--prompt`。
- amp 的 `--mode` 不在 New Agent 给选项（任务文件「不做」）。

## 用户试用后修复（10-14）

- 用户 10-14 重开试：“下面第二个下拉框一点就崩溃了”（New Agent 底部选 agent 的那个，附崩溃报告：`kind_icon::of` 下标越界）。
- 原因：`kind_icon.rs` 里本机原图的表写死 4 格，加 amp 时只改了 `KINDS`（5 个），画 amp 图标时越界。原先 kind_icon 的测试只走了剪影那条路，没经过这张表。任何地方要画 amp 的图标（New Agent 的选单、amp agent 的卡片和标签）都会崩。
- 修：表长改成 `KINDS.len()`，以后加一家不会再漏。新测试 `every_kind_has_an_icon`：每家都经 `of` 取一次图标；修前失败（`the len is 4 but the index is 4`），修后通过。
- `cargo test --all-targets`（库内 505 条）、clippy、fmt、`git diff --check` 通过。没起窗口，留给用户重开再点一次。
- 用户 10-14 重开后：“我重开了，选单能打开了，我也测试了，基本可以用。”
