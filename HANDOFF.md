# 交接

2026-10-15 起本文件只记项目级的账：现状、下一步、悬而未决。每次会话做了什么、用户怎么说的，由 cairn 的接续摘要记（本仓库 10-10 起采用 cairn），新会话开局自动注入；每件活的经过在 `docs/任务/<编号>…md`（完成记录、主控审查）和 git 的「合并」「收尾」提交里；更早的会话经过在 git 历史里（`git log -p HANDOFF.md`）。设计和理由在 `docs/DESIGN.md`。

## 现在在哪

- **没有进行中的活**：没有任务 worktree。5090 上的测试 agent `paddock/test-pi` 用户 10-17 自己关了（在远端开 pi 要经远端 `zsh -l` 跑 `$HOME/.local/bin/corral start`，非交互 ssh 找不到 nvm 下的 pi）。ranch 的 R12 草稿（派活规则和路由策略）主控提交在 ranch 本地 main、没推送，用户说自己处理。工作区里未提交的草稿是 P5-54、P5-111、P5-149、P5-115、P5-125、P5-144、P5-146（看板 DRAFT，有意留，不要 `git add -A`）。P5-111（调研 OSC 7501）10-20 派出后用户叫停（“有风险，而且收益不行”），范围已定为任务文件里那 5 条；用户 10-20 在看板上又把它退回草稿。
- `~/Applications/paddock.app` 是 P5-164 的版本（10-21 从分支装，代码和合并后的 main 一样；含 P5-147、P5-163），本机 Apple Development 证书签名。以前各版含哪些活、用户看没看过，在各任务文件和收尾提交里；用户还没看的列在「用户用到时顺带看」。
- 已合并：第一至第三阶段、迁移 M0–M3、P4-1～P4-3、P5-1～P5-80、P5-82、P5-85～P5-89、P5-90、P5-90a、P5-91～P5-105、P5-107～P5-110、P5-112～P5-114、P5-114a、P5-116～P5-118、P5-118a、P5-119、P5-121、P5-121a、P5-122、P5-123、P5-124、P5-124a、P5-124b、P5-124c、P5-126、P5-127、P5-128、P5-129、P5-129a、P5-130、P5-131a、P5-132、P5-133、P5-134、P5-135、P5-136、P5-137a、P5-138、P5-139、P5-120、P5-142、P5-141、P5-143a、P5-145、P5-143b、P5-143c1、P5-143c2、P5-143d1、P5-143d2、P5-143d3、P5-143e、P5-148a、P5-148b、P5-148c、P5-150、P5-151、P5-152、P5-153、P5-154、P5-106、P5-155、P5-156-P5、P5-157、P5-158、P5-159、P5-160、P5-161、P5-162、P5-147、P5-163、P5-164（P5-137 是 agy 没做完、改由 P5-137a（omp）做，不做；P5-95b 已被 P5-95c 撤；P5-131b 是 agy 做的比较落选，不合并）。P5-83 已收尾（讨论记录）；P5-84、P5-81 不做；P5-52 只查明原因、改法用户暂不定。
- **远程机器（P5-90 甲）**：用户本机 `~/.config/paddock/config.toml` 已加一台 `[[machines]]`（名字 `5090`，ssh 别名在本机配置里，不进仓库）；只有 P5-90 之后的版本认这一项，旧版会报配置错。远端 corral 用 `$HOME/.local/bin/corral`（ranch R10 部署）。
- 重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录 `.target/main`）。打包默认选本机 Apple Development 身份，可用 `PADDOCK_SIGN_IDENTITY` 指定；退回 ad-hoc 时会提示授权会失效。
- **paddock ctl 已部署**：`~/.local/bin/paddock` 链到 app 里的程序，重新打包后不用重建；技能正文改了要重跑 `paddock install-skills`（装在 `~/.claude/skills/paddock`、`~/.agents/skills/paddock`、`~/.gemini/config/skills/paddock`），已开着的 agent 要重开才看到。
- 全局 `~/.local/bin/corral`、`ranch` 是 ranch `versions/e62ee85`（10-21 部署 R34：派活默认分工表换成 low/middle/high/critical × design/other 八格，加 review-codex、review-claude、amp、agy 共 12 格，旧格子名全没了；Claude Code、Codex 可选强度只剩 high、xhigh。命令和 JSON 结构不变；paddock 核对过，Settings › Dispatch 按数据画，代码里没写死格子名，旧名只在 `dispatch.rs` 的合成测试数据里）。之前 `versions/6907a32`（10-21 部署 R36、R37：只改 corral、corral-dispatch 两份技能的文字，命令、JSON、退出码不变，paddock 不用改；主控派活的新规矩见 ranch `docs/派活路由规则.md` 8.1：被委派的 agent 把要拿的主意写在回复里结束这一轮，主控照技能第 8 节自己定、`corral send` 回去，隐私、花钱、改批准过的设计、删除、推送、关 agent 或拿不准时先问用户；送前查 `corral status`，用户插过手就不送。中间 `versions/c8d57bb` 是 R35。更早 10-19 的 `06e2c27` 含 R22、R24、R25：人按 Esc 打断被委派的 agent 时，`status` 多 `interrupted`、`wait` 返回 `interrupted`、`reply` 在被打断的一轮报 `no_reply`、`send --after` 跳过这一轮；Claude Code、Codex、pi、amp、omp（R25）认得出，agy 认不出。paddock 不用改：被打断后卡片的回复预览空着，下一轮正常结束再出来；要不要显示“被打断”，以后另开任务定）。amp 插件（ranch R6）、agy 插件（`~/.gemini/config/plugins/corral`）在本机装着，用户定留着；两家的真图标放在本机 `~/.config/paddock/icons/`，不进仓库。
- **fence（sandbox 的墙）**：ranch R17b，10-19 部署（`~/.local/bin/fence`，ranch `versions/7fbb76d`），只有内置档案 `dev`；Claude Code、Codex、pi、omp 能套，amp、agy 不能；5090 没装。paddock 只调 `fence --version/profiles/check/denials` 和 `corral start --sandbox`（P5-120）。R18（`corral child-sandbox`，给主控设它派出去的套哪个档案）、R19、R20 已部署（`versions/aa3676b`）：dev 档案放行 agent 自己临时目录下的 unix socket，四家墙里的 `TMPDIR` 都是各自的 `~/.cache/fence/tmp/<实例>`；paddock 全量测试在墙里两家都过（P5-142）。
- **cairn 0.5.0 加了“项目的账”**：10-16 照本文件起了 29 条（`cairn ledger` 看，`cairn ledger log` 看旁路模型每笔改动和依据）；和本文件并行用一周左右，看自动记得准不准，之后由用户定本文件和 AGENTS.md 里维护它的规矩删不删，这期间都照旧。面板展示 P5-118 已合并（Cairn 标签 `Notes | Ledger` 切换，只读）。cairn 0.6.0（10-16 装）加了经验沉淀（kind `lesson`），P5-121 加了第三颗 `Lessons`。cairn 0.7.0（10-17 装）：数据库版本 5（加 `ledger_runs`，0.6.x 读不了，退回要换回备份），每次记账留一行结果（`cairn ledger runs`），`status` 末尾多“最近记账的会话”；DESIGN §7.1 已有字段没变，面板不用改；`status --json` 的 `ledger.recent_sessions` 不在公开约定里，面板要用先请 cairn 写进约定。
- **cairn**（没什么可接续的会话不写记录）：Claude Code（`~/.claude/settings.json` 4 个 hook）和 Codex（`~/.codex/hooks.json` 4 处，直接启动的 Codex 要用户到 `/hooks` 里信任）都装了；本仓库已 `adopt`。接续摘要由 cairn 后台写，主控不用每回合 `cairn save`，用户要留记录时才存。paddock 调用的 cairn 命令（`status --json`、`show [<ID>] --json`、`list --json`、`adopt`）是 cairn DESIGN §7.1 的公开约定。退回：`cairn unadopt`；卸掉：`cairn uninstall --agent claude|codex`。
- 上下文：`AGENTS.md`（规矩，含「开发方式」里的看板约定）、`docs/DESIGN.md`（§13 是界面改版的全部决定和用户原话）、`docs/背景与决策记录.md`、`docs/设计稿/`、`docs/调研/`。

## 用户交代、一直沿用的

- 用户的 Linux 实测机（5090）：暂时只拿 pi、omp 做实验，任何情况下都不运行那台机器上的 Claude Code 和 Codex，pi、omp 也不委派任何活。是暂时的，放开以用户明说为准；远程支持照样完整做。主机名、地址不进仓库。
- 主控不在 agent 会话里起测试窗口（截图、实测），样子留给用户看。
- auto 模式拦下主控改 `~/.local/bin`：ranch 部署（切链接、`upgrade --all`、装技能）由主控打包、核对，用户在普通终端里跑。
- 开 agent 靠用户加的允许规则 `Bash(corral start:*)`，命令开头不加 `cd`。

## 下一步

1. **P5-161 Settings › Dispatch** 已合并收尾（ranch 转来，Claude Code opus high 套墙做，一次过，用户看过“可以”）：连线图看分工表，改一格走 `ranch dispatch edit`／`reset` 带版本防冲突。建议改（没做）：换选别的格子时没保存的改动直接丢、不提示，看用户用着要不要。和用户讨论要一条条细谈，不要一次抛一堆问题。 **P5-162** 已合并收尾（paddock 不是当前应用时所有窗口的颜色本身去色、不盖灰，没键盘的光标一律空心；Claude Code opus high 套墙做，返工 1 次：盖灰没质感改去色、改按应用判断，最后主控按用户“有点浅”调深）。建议改（没做）：Browser 网页、agent 品牌图标、New Agent／About 小窗不去色。
2. **P5-54** Browser 选中元素、Changes 选中 diff 写上要怎么改后送给 agent（优先：高，草稿）。主控提议（用户还没表态）：样稿主控自己做；实现拆成 P5-54a Changes、P5-54b Browser（WKWebView 注入 JS 加 WKScriptMessageHandler，代码库里还没有），a 合并后再派 b；测试只用假 corral。下次从列出待定问题开始。
3. **P5-106 看板 LOOP** 已合并、已装、已收尾（Claude Code high 套墙做，返工 1 次：卡住改成主控和开发 agent 都不在干活才计时）。用户 10-20 用它走了三轮：P5-155；P5-156-P5 加 P5-157（一件派出、一件主控自己做）；P5-158（Loop 这块重排成框顶 Now、下面 Next、去掉状态行，Claude Code sonnet high 套墙做，分工表第 8 条试用第 1 件，一次过，用户试过“可以”）；P5-159（Changes 加 History：当前分支提交图、点一行看 diff、Copy hash、Open task；和用户逐条定、画样稿后用户点名交给 omp 不套墙做，一次过，用户试过“可以”）。P5-160（Settings › Sandbox 显示查资料用的 MCP 服务器，ranch 转来的需求，用户点名 omp 不套墙做，一次过，用户看过“可以”）。都已收尾。换成要 key 的服务商照 ranch `crates/fence/README.md` 手改 `~/.fence/`；界面里换要先请 fence 加命令，用户还没要。P5-158 的建议改：Now 的转圈不跟“减弱动态效果”，用户 10-20：“先这样”。任务文件名编号后别紧跟另一个编号（`P5-156-P5-155…` 被看板认成 `P5-156-P5`）。
4. **派出去的 agent 套 sandbox**：用户 10-19 要的效果是 ranch、paddock 默认都不套，在 paddock 里给主控设上后它之后派出去的才套、随时能改（做法甲，ranch R18 已做）。P5-140 已实测：Claude Code、Codex 都能套墙干活。**P5-142** 已收尾：墙里全量测试两家都过（测试 agent 的编译目录要在它自己的路径下编过，克隆来的先 `cargo clean -p paddock`）。**P5-141** 已收尾：主控卡片展开后 `Delegates` 一格设它之后派出去的套哪个档案，仓库顶层 `.fence.toml` 放行 `paddock-worktrees/.target`（墙里的 agent 不用再带 `--env CARGO_TARGET_DIR`）。还没做：派活规矩里套墙的审查 agent 怎么写审查记录（墙里写不进主仓库工作区，建议写在审查 worktree 里由主控搬回），等用户要用套墙派活时再提。
5. **P5-143 优化终端**：五批（a、b、c1、c2、d1、d2、d3、e）都已合并收尾，P5-143 整体收尾（10-20）；同步输出超时缺口、连字和按窗格字号、终端图片和着色器、P5-143a 专门实测，用户 10-20 定不做，有需要再另开任务。
   - **P5-154** 标签切换、侧栏卡片换位、状态涟漪已合并、已装、已收尾（Claude Code high 套墙做，主控审一次过；建议改：卡片滑时分组标题和托盘直接跳，用户实测没提）。
   - **P5-148 终端和界面的过渡与信号**（用户 10-20 定，按 a、b、c 依次做，都改 `window.rs`）：**P5-148a** 过渡已合并、已装、已收尾（Claude Code 套墙做，主控直接审）。**P5-148b** 进度条和响铃已合并、已装、已收尾（Codex high 套墙做，主控直接审，一次过；建议改：输出多过一遍 VTE 解析，以后看能否合成一遍）。**P5-148c** 选中后送给 agent 已合并、已装、已收尾（Claude Code opus high 套墙做，主控直接审，一次过）。P5-148 三件都完了。**P5-149** 终端按命令分段是草稿，优先：低，不排。已有的光标滑动、滚轮缓动不跟 macOS“减弱动态效果”：**P5-150** 已修、已合并、已收尾（omp 不套墙做）。**omp 套 fence `dev` 墙起来十几秒就停、没回复**（P5-150 第一次派时），用户 10-20 让转给 ranch，已 `corral send ranch/main` 交过去，等 ranch 查（paddock 这边不用回复）；浮层淡出约 84 ms 内仍接点击，暂按可接受。
6. **P5-144 基于 pi 做自己的 agent**（草稿，长期）：用户 10-19 定先不调研、不排期。
7. **P5-146 把终端拆出去做自己的终端**（草稿，长期，优先：低）：用户 10-19 提的想法；先在 paddock 里做完 P5-143 b、c 再说。草稿里列了要想清楚的（和别家差在哪、先拆成单独 crate 还是独立仓库、要不要现在就把“终端层不依赖侧栏、corral、看板”写进 DESIGN）。
8. **P5-125 递归自我改进（RSI）**（草稿）：五个待定问题在草稿里（改什么、素材、多久回头看、谁批、要不要界面），等用户定。
9. **P5-147 Settings › Chat 已合并、已装、已收尾**（用户 10-21 重开看过“可以”）：10-21 晚些 pony 0.1.0 已在本机，Chat 页显示 `not set up`（没有 `~/.pony` 配置），用户截图看过，显示正确。等 ranch R38f 上线（建 Slack 应用、写配置、装服务）（机器人主体在 ranch R38），用户顺带看一眼连接和频道的真实样子。
9a. **P5-163 干活中标签的跑马灯边已合并、已装、已收尾**（用户 10-21 实测“cool，看了，可以”）。用久了嫌吵：调 `app/src/window_orbit.rs` 的 `DIM`（没选中的亮度）、`PERIOD`（一圈多久）。
9b. **P5-164 左侧栏底下的 Idle workspaces 改成淡托盘已合并、已装、已收尾**（omp 不套墙做，返工 1 次，用户 10-21 重开看过“可以”）：闲着的 workspace 装进列表末尾一个更淡的托盘（标题 `Idle`），每行右边写仓库名，`⋯` 悬停才出；合成行展开后标题 `No agents <个数>`、下面只写仓库名。浅色主题样稿没画，用户用到时顺带看。
10. 以后：Changes 第二步（行上评论发给 agent、暂存、撤销，可能和 P5-54a 合并考虑）；Kanban 能拖来纠正（排在后面）。

## 用户用到时顺带看（不用决定、不挡任何事）

主控没法替用户试的只有真实悬停、拖动、键盘、输入法：

- 等用户点一次的：Settings › Agents › Integrations 真点一次 Install／Update（P5-94；本机现在都最新，没有按钮，要看可先 `paddock install-skills --remove --yes`）；agy 一行点 Install 装上 paddock 技能（P5-104）；New Agent 的 Codex 默认是不是 `gpt-6-astra` `high`（P5-100）；amp 没用真账号实测过（P5-98，要用用户账号，先问）。
- 主控没起窗口看过的新界面：P5-103 收件箱（高度跟内容是最可能要调的地方）；P5-99、P5-101、P5-102 New Agent 底边和模型选单；P5-105 workspace 浮层每个仓库一行、同名的带淡色上级目录、浮层开着时新进来的仓库自动勾上；P5-95 磨砂透出来的样子、标题栏上的字和按钮看不看得清、拉窗口时边上闪不闪。
- P5-91 workspace：收起时一摞的层次和间距、叠卡的大小和歪的角度、只有 1 个和没有 agent 的 workspace 收起的样子；展开时槽的深浅；浅色主题；透出系统材质时槽里卡片的亮度；界面字号调大、侧栏拖窄时。
- P5-89 仓库收起和 workspace：点仓库小标题收起和展开、悬停才出的箭头；`New Workspace…` 浮层里 `Choose folder…` 回来以后浮层还在不在；中文输入法下起名字（选字时的 Return 归不归输入法）；workspace 标题上的 `⋯` 菜单（就地改名、`Add repositories…`、点两次的 `Remove workspace`）、卡片详情里的 `Workspace` 一格；左侧栏收成窄条时的分隔线和悬停说明；界面字号调大、侧栏拖窄时挤不挤。
- P5-118、P5-118a Cairn 的 Ledger 页：靠左小胶囊切换和凸起框的样子、条目点开、右侧栏最窄 426 和字号调大时挤不挤。
- P5-133 Cairn 的 Ledger、Lessons（主控没起窗口看过）：一行一条和右边日期、组头点了收起（默认只开 State，重开 paddock 还记得）、过滤框（命中高亮、`N of M`、`×`、Esc）、⌘F 要先在 Cairn 标签里点一下才进过滤框；**要定**：点 Cairn 标签后键盘离开终端，回终端要再点一下，接不接受。
- P5-134 左侧栏（主控没起窗口看过）：agent 全停的 workspace 收到列表末尾 `Idle workspaces` 小组、点一行回到上面展开、点标题送回底下；有 agent 的 workspace 里两个以上没 agent 的仓库合成一行、点开逐行；悬停 `⋯`、就地改名、浅色主题、窄条。
- P5-120 sandbox 界面（Claude Code 做，主控没起窗口看过）：New Agent 皇冠左边的锁（选 `dev`、Codex 的警告、Controller 时变灰、检查没过 Start 灰）；预设记住 sandbox；套墙 agent 卡片的小锁和黄色数字、展开后 Sandbox 一格、被拦记录浮窗（`Network & writes`／`All`）；Settings › Sandbox 只看。退回旧版 paddock 前要去掉配置里预设的 `sandbox` 字段，否则旧版读不了配置。
- P5-129a 仓库名 11.5pt；P5-130 胶囊提醒记号、切换栏胶囊改淡、右侧栏说明卡居中（10-17 那版装上，用户还没看）。
- P5-150 光标滑动、滚轮缓动跟系统“减弱动态效果”：打开“减弱动态效果”后光标不滑、滚轮不缓动（用户说直接合并，没试）。
- P5-135 New Agent 打开时：项目还没有 main 亮 Controller、已有 main 亮 Developer。
- P5-136 卡片展开后的详情（agy 做，主控没起窗口看过）：展开时不画位置行、路径和小药丸在详情第一段、两条细分隔线、格子少了（Status 带时长、Windows 为 0 不出现）、标签小号大写、Workspace 无常驻底色、Stop 描边；格子标签 9.5 号再淡一档，看会不会太淡。agy 这次 corral 状态一直报 `starting`、没留 reply（插件没报状态），完成靠用户告知。
- P5-137a Cairn 摘要（omp 做，主控没起窗口看过）：Next session 摘要不再用 GPUI 的两行截断（中文被估窄、右边被切），改成整段换行、框高两行、多的藏起来、第二行末尾没有 `…`。同类“中文 + 最多几行”的截断在看板标题、收件箱可能也有，用户 10-17：“先不查了”。
- P5-138 ⌘Q 退出（Codex 做，主控没起窗口看过）：开着几个 agent 窗格时，退出后 Dock 图标应该很快消失，不再多挂约 3 秒乘窗格数。交叉审查建议 1 没做：补一条经“窗口收集所有窗格”这条链的回归测试（现在的测试只测到 PTY 一层），要不要另开一件用户还没定。
- P5-124、P5-124a、P5-124c 远程开 agent（主控没起窗口看过，也没在 5090 上真起过）：P5-124c 的 Command 行（改命令、Reset）、`+ Custom…` 和起成功后变出的胶囊（悬停 `×`）、宽度 560；切到 5090 后侧栏空着时中间的 `New Agent` 按钮、底部菜单 `New Agent…`、⇧⌘N、胶囊右键 `New Agent on 5090…`；对话框里 pi、omp 查得出来不（经远端登录 shell）、目录默认 `~`、最近目录、名字留空用 `<目录末段>/<种类>`；起好后窗格里 attach；目录不存在时错误显示在框里；精致四条（对齐、悬停焦点等状态、Tab／←→／⌘↩／Esc、出错时不跳）。在 5090 上照规矩只开 pi、omp。
- P5-121a Cairn 切换（凹槽外框、实心胶囊）：主控没起窗口看过，浅色主题是按规则推的，要看一眼。
- P5-121 Cairn 的 Lessons 页（第三颗胶囊）：要等旁路模型记下第一条经验才有内容；用户选了不往真实的账里手工加。
- P5-119 Settings › Instructions：六家切换、Markdown 显示、Edit／Review changes…／Save 和备份、Create file；编辑、保存按用户的话没有测试，也没人点过，第一次保存建议先在不要紧的那家（如 pi）试。
- P5-79 Cairn 记录列表：摘要两行／三行截断、高亮卡片、右侧栏拖到最窄 426 时每条第一行挤不挤；带更正的记录点开后有没有 `Corrected` 那段；`Show older` 要记录超过 50 条才出现。
- P5-80 按仓库分组：刚启动或新 agent 刚出现时会先按名字前缀放、读到仓库后挪一次组，看着突不突兀；⌘P 里前缀和仓库名不一样的 agent 那行多写一个组名。
- P5-55、P5-78 Cairn 标签：右侧栏拖到最窄（426）、界面字号调大时四个标签、两个按钮和状态行挤不挤；在没采用的仓库里看 `Adopt…` 和确认卡片（真点 `Adopt` 会采用那个仓库，`cairn unadopt` 撤回）。
- P5-77c、P5-77d 名字签（两枚都在输入框右上角）：够不够醒目（不够可加样稿 D 里“窗格的边同时亮一下”）；输入折到多行时落在哪；Codex 里落在哪（找不到空行会跑到右下角）；实心强调色在各主题下刺不刺眼。
- P5-61 Agent shell（只支持 zsh、bash）；P5-59 agent 窗格标题行里会话题目（`corral attach` 不转标题的话改读 `corral status` 的 `title`）。
- P5-66 退出时只在 shell 有程序时才问；P5-56 Settings 改了立即生效、Undo、“not installed”。
- 10-07 那批：拖缝调大小和双击；⌘Z／⇧⌘Z 和输入法；标签“+N”菜单；四套新主题（Rosé Pine 光标偏暗）；ctl 的 busy、`--focus`、`--attach`；格子图扫光、光晕、呼吸；一键暂停（会冻住包括主控在内的全部 agent）。
- Browser：P5-28c 照 `docs/任务/P5-28c-Browser网页策略与查找.md` 完成记录末尾 8 条清单（第 1 条本地地址最要紧）。Kanban 状态推得对不对、Needs you、Clear 的确认。

## 悬着

- P5-131 遗留（用户 10-17：“两件都不处理吧”）：agy 落选的分支 `p5-131b-kept` 留着不删（没合并，删要 `-D`）；P5-131a 的三处建议改不做——格子图存失败不重试、菜单两句提示只显示前一句、存失败后再选已在列表里的仓库不重试保存。
- P5-118、P5-118a（Cairn 的 Ledger 页）是主控自己做的，合并前没让别的 agent 交叉审查；用户 10-16 说先不开审查。
- P5-119 遗留：编辑时直接关掉 Settings 窗口，改动不提示就丢（交叉审查建议改）；编辑框不自动滚到光标处。用户 10-16：“暂时这样，这些都是小问题”，不另开。保存不加锁，改名前最后一次核对到改名之间别处的改动仍会被覆盖（已写进 DESIGN）。
- **别强制退出 paddock、别点 Dock 上多出来的 paddock／`exec` 图标**：从 paddock 里开的 agent 都在 paddock 的 coalition 里，强制退出会把它们一起结束（P5-52；主控建议测试实例用 Accessory 策略，用户暂时不改）。正常 ⌘Q 没事。
- P5-90 遗留（用户 10-16：“先把功能做完，之后碰到问题了再修。”）：断网、断线、重连没测过（用户：“断网那个有时间再测试吧”，不让主控做任何断网测试）；退出 paddock 时正在跑的远程查询进程可能多留最多 15 秒（交叉审查复核建议改）；只读条靠 `attached` 总数判断，多个客户端时可能过早消失。建议用户给 5090 的 sshd 设 `ClientAliveInterval`，Mac 睡眠或断网后残留连接能自己清掉（用户自己改）。
- 收件箱回复以 `-` 开头的送不出去（本机和远程都是）：要 ranch 把 `corral send … -- <文字>` 的 `--` 写进公开约定后 paddock 固定带上，等用户点头（P5-87）。agent 输入框里有没提交的字时，回复会接在后面一起提交。
- `AGENTS.md` 里“只通过 `corral`、`ranch` 命令打交道”那句没把 cairn 写进去（paddock 已在调 cairn 的公开命令），要不要补问用户。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问运行中的 shell（GPUI 没有拦截）。Claude Code 带的 `caffeinate` 冻住后“不睡眠”断言仍在，暂停 agent 不等于让 Mac 能睡。
- Saddle 用户基本不用了；Saddle 仓库里的 `t76-*` 分支、worktree 由 Saddle 主控处理。
- 建议改未排：`cairn_view.rs` 里画 Markdown 行内样式的 `styled` 和 `kanban_view.rs` 里那份重复；右侧栏最窄 426 嫌宽的话要另想办法（比如窄的时候标签只留图标）（P5-55）；Cairn 记录点开时不自动滚到它、记录行没有悬停底色（P5-79）；名字签在查找条（⌘F）开着且光标在最上面几行时会盖住查找条、窗格窄到放不下名字时右端被裁（P5-77）；任务弹框正文写到框底不自动滚到光标处、弹框开着换主题不变色（P5-60b）；New Agent 的弹出框不能用键盘上下选（P5-37）；看板切仓库的菜单没有滚动、Kept 里暂时不在的目录会在下次加或移时被去掉、列表存不进去不提示（P5-85）；Drop task 的原因输入框全看板只有一个（P5-86）；同名仓库上级目录缩短后可能一样（P5-105 S1，用户定先不做）；`paddock ctl open --prompt … -- amp` 会被 corral 拒（P5-98，用户定暂不做）；DESIGN §13 的 P5-20 条重复了一遍；卡片悬停不做渐变（P5-51）。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式（只做了本机签名，不公证、不分发）。Metal 工具链不装，靠 `runtime_shaders`（P5-62 量过代价很小）。
- 主控做法备忘：
  - 自己做的小活：建 worktree 后 `cp -cR ../paddock-worktrees/.target/main ../paddock-worktrees/.target/<分支>`（APFS 克隆）给它一份编译目录，只做增量编译；收尾一并删掉。
  - 派活：“不要做”里写明不用 Python、命令里不用 `rm`、不用 `sh -c` 包长命令、前台命令别带等标准输入的东西；`corral start --unique` 会给名字加 `-1`，后续用返回的名字；新 worktree 第一次开 Codex 会卡在“是否信任这个目录”，要用户去点。
  - `send --after` 的第一个参数是**收件人**（提醒自己写 `corral send "$CORRAL_NAME" … --after <对方>`）；corral 没有撤销提醒的命令。提醒可能在合并之后才到：核对 `corral status` 的 `state_started` 没变就是旧提醒；收到提醒先看 `git log` 和 `corral read` 再动。
  - 派出去的 agent 用提问对话框停下来时（`corral status` 是 blocked、`last_tool` 是 AskUserQuestion）：`corral read` 看尾部的问题；`corral keys <名字> esc` 关掉对话框后 `corral send` 仍被退回 7，用 `corral keys <名字> "text:…"` 再 `corral keys <名字> enter` 把回答送进去；之后重新挂提醒。
  - 关 agent 前确认它 idle、`attached` 为 0（paddock 窗口里显示着的会是 1）；release 构建很快结束时，核对产物时间晚于合并再安装。
  - 测试截图用临时 HOME、假 corral、`--bounds` 和预写布局（现在先不起窗口，见上）；corral 的 socket 路径有长度上限，单独的 `CORRAL_HOME` 放 `/tmp` 下的短目录。
