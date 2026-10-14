# 任务：Settings 里一键配置 agent

2026-10-14，paddock/main 记。
依据：用户 10-14 谈 P5-88（集成 agy）时：“我的理解是我想做成settings。然后里面列出一些agent，然后类似用户一键适配这种。”主控列了适配分几块、要动哪些仓库后，用户：“这样，我再把scope放低，这个作为一个单独的任务。就是settings里面用户能够一键配置agent。不和这个agy的任务缠绕在一起。另外cairn也不在本次的scope中。我理解cairn就是一个独立的记忆系统”“先开任务。之后再讨论”。之后：“其实我的最终目的是要想herdr的settings里面的一个agent适配的功能一样。”“那么我们是不是也可以这么做？（限于omp，pi，claude code以及codex这种）。”主控列了方案和四件要定的事（先出样稿；不做卸载；paddock 自己的技能认 `CLAUDE_CONFIG_DIR`；谁来做），用户：“按你的建议，先出样稿，你自己做”；看过样稿：“样稿可以，写任务文件吧”。经过见文末「讨论记录」。
依赖：ranch R4（`corral integration status`，ranch `b0e3b6f`，10-14 已部署）。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-94-integrations`，分支 `p5-94-integrations`（开工时从 main 建）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-94-integrations`。
- 动 `app/src/settings_view.rs`（Agents 页加一节、确认框）、新的不依赖 GPUI 的模块（读命令输出、算每行状态、该跑什么）、`app/src/skills.rs`（按位置装、认 `CLAUDE_CONFIG_DIR`）、测试、`docs/DESIGN.md` 和本文件。

## 样稿
`docs/设计稿/P5-94-Integrations/`（原稿在 claude.ai 私有画布「Integrations样稿」）：`1-Agents.dc.html` Agents 页加 Integrations；`2-Confirm.dc.html` 点 Update 后的确认框；`3-States.dc.html` 第一次用、Checking…、Not found、corral 找不到。

## 要做的
Settings 的 Agents 页，在 corral 一节下面加一节 `Integrations`，像 herdr 的 integrations 标签，照样稿：
1. **名单**：每家 agent 一行，名单和顺序照 `corral integration status` 的输出（现在 claude、codex、pi、omp），不在 paddock 写死。显示名 Claude Code、Codex、pi、omp，不认识的种类照原样显示。
2. **Agent 列**：名字，下面一行是版本（`version`）；找到了但拿不到版本写 `Installed`；没找到（`path` 为空）写 `Not found`，整行变淡，Status、Skills 两列写 `—`，没有按钮。
3. **Status 列**：`hooks` 是 `launch` 显示绿色的 `Built in`，没有按钮；不认识的值照原样显示，不报错。
4. **Skills 列**：三个技能 `corral`、`corral-dispatch`、`paddock`，看它们在这家会读的位置（status 里这家的 `skills[].target`：claude＝Claude 配置目录、codex＝`~/.agents`）装没装，合成一个状态：
   - 都最新：`Up to date`。
   - 有没装的或内容不同的：一个 `Install`（全没装）或 `Update` 按钮，旁边一句说明哪个技能缺了／变了（样稿的 `paddock changed`）。
   - `foreign`（符号链接、不是它装的）的不动、不算进按钮；只剩这种时显示 `Not managed`。
   - 状态来源：`corral` 用 integration status 的 `skills[].status`；`corral-dispatch` 用 `ranch dispatch install-skills --dry-run` 的 `items`（`agent` 就是位置）；`paddock` 用 paddock 自己的 `skills.rs` 空跑。
5. **点按钮**：先弹确认框，写明要写哪几个文件、哪些是替换，以及这个位置还有哪几家也读（样稿 2）；确认后在后台跑 `corral install-skills --target <位置> --yes`、`ranch dispatch install-skills --target <位置> --yes`、paddock 自己按位置装，只装这家缺的或变了的那几个技能、只装这家读的位置。装完重新读一遍；同一位置的别家跟着变成最新。
6. **什么时候读**：打开 Agents 页、点 Refresh、装完之后，在后台读（一次约 1.3 秒，最多三秒多）；读的时候标题右边显示 `Checking…`，上一次的结果先留着。不放进每秒刷新。
7. **读失败**（corral 找不到、输出不认识）：这一节只显示一句红点错误和 Refresh，不影响页面其他部分（样稿 3）。
8. **不做卸载**：只有 Install／Update（`ranch dispatch install-skills` 不能 `--remove`）。
9. **paddock 自己的技能认 `CLAUDE_CONFIG_DIR`**：设了就装到 `$CLAUDE_CONFIG_DIR/skills`，没设照旧 `~/.claude/skills`，和 corral 一致；`paddock install-skills` 命令行同样。
10. `docs/DESIGN.md` 加 P5-94 一条；§3 记上 paddock 新调的公开命令（`corral integration status`、`corral install-skills`、`ranch dispatch install-skills`）。

## 怎么算做完
- 用户：“就是settings里面用户能够一键配置agent”“想herdr的settings里面的一个agent适配的功能一样”“样稿可以，写任务文件吧”。
- 自查：`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过。读输出、算每行状态和该跑什么的逻辑用合成 JSON 和假命令测（先写测试看它失败再实现）；装技能的测试只用临时 HOME，不碰真的主目录。
- 装上给用户看；主控不起测试窗口、不截图，样子和点按钮装技能留给用户实际试。

## 不做
- 不碰 agy、amp（P5-88 和以后“全局安装”那一档）；不碰 cairn；不做读屏兜底；不做卸载。
- 不改 ranch，只调它的公开命令；不读 corral、ranch 的内部状态。

## 主控另拿的主意（用户已同意）
1. 调哪个 corral：用 Settings 里 `corral command` 那一项（和 paddock 别处一样）；ranch 用 PATH 上的 `ranch`。
2. 找不到 `ranch`：`corral-dispatch` 不算进按钮，Skills 旁边小字写 `ranch not found`，其他照常。
3. 装完在 Settings 底部的消息条报一句（如 `Updated 1 file for pi`），失败红字带原因，沿用现有消息条。
4. 窗口窄到三列放不下时，Status、Skills 挪到名字下面一行。

用户 10-14：“四条都同意，开始吧”。

## 讨论记录（10-14，由旧到新）

### 现状（主控 10-14 查的，只读）
- Settings 的 Agents 页现在只有 corral 一节（`refresh_ms`、`corral`，`app/src/settings_view.rs` 的 `AGENTS`）。
- 装技能已有三条公开命令，输出同一种 JSON（`ok`、`action`、`dry_run`、`written`、`warnings`、`items[{agent, path, status}]`），都支持 `--dry-run`（只看不写）、`--yes`、`--remove`：
  - `corral install-skills [--target all|claude|codex] [--project DIR]`：技能 `corral`（开别的 agent、送话、取回复）。
  - `ranch dispatch install-skills [--target all|claude|codex]`：技能 `corral-dispatch`（当主控拆任务派活），三个文件。
  - `paddock install-skills`（paddock 自己的，`app/src/skills.rs`，没有 `--target`）：技能 `paddock`（操作 paddock 界面）。
- 三条命令的目标都只有两家：Claude Code（`~/.claude/skills`）和 Codex（`~/.agents/skills`）。本机 10-14 三条空跑都是 `same`（已装、最新）。`status` 除 `same` 之外还有哪些值（没装、旧了、不是它装的……）没查，开工前查 ranch 的公开约定（`corral guide` 指向的 `docs/CONTRACT.md`）。
- 只调这三条公开命令，不读 corral、ranch 的内部状态，不改 ranch。

### 主控的设想（早先，已被下面的方案取代）
- Settings 的 Agents 页加一节：每家 agent 一行，写出命令装没装（版本），三个技能各自是已装、有新版还是没装；每个技能一个 Install／Update 按钮，每行一个全部装好。状态用 `--dry-run` 读，点按钮才 `--yes` 真装。

### 早先列的开工前要定（已并入下面的方案）
1. 列哪几家：主控建议只列 Claude Code、Codex（装技能的命令只支持这两家；pi、omp 列出来没东西可配）。
2. 要不要移除按钮（三条命令都能 `--remove`）：主控建议要，点一次再确认一次。
3. 先出样稿再写要做的：主控建议先出样稿。
4. 除了这三个技能，“一键配置”还包括什么（用户的原意可能更宽，讨论时问）。
5. 谁来做、优先级：用户没说，先不写。

### 不做（早先定的）
- 不碰 agy（那是 P5-88）。
- 不碰 cairn（用户：“cairn也不在本次的scope中。我理解cairn就是一个独立的记忆系统”）。
- 不改 ranch；要它改的写成需求在 ranch 仓库做。

### 主控早先的做法和风险
- 再查到的（只读）：`status` 装的时候有 `create`（没装）、`same`（一样）、`overwrite`（内容不同）、`foreign`（是链接或不是它装的，不动）；卸的时候有 `absent`、`remove`、`foreign`。`ranch dispatch install-skills` 没有 `--remove`。`corral guide` 里没写 install-skills 的输出格式（不在书面公开约定里）。corral 装 Claude 的技能时认 `CLAUDE_CONFIG_DIR`，paddock 自己的 `skills.rs` 不认。paddock 从 Finder 启动时已经取登录 shell 的 PATH（`main.rs`），找得到 `~/.local/bin` 下的命令。
- 要改的（只在 paddock）：不依赖 GPUI 的新模块读三条命令的 JSON、算每家每个技能的状态和该跑什么命令（带测试，用假命令）；后台跑 `claude --version`、`codex --version` 看装没装；`settings_view.rs` 的 Agents 页加一节；paddock 自己的技能直接调 `skills.rs`；DESIGN §3 记上 paddock 新调的两条命令。
- 风险：①输出格式不在书面约定里，ranch 改了 paddock 会读错（可请 ranch 写进约定）；②`overwrite` 分不清“旧了”“用户改过”“装的比 corral 自带的还新”，点 Update 会覆盖用户的改动；③corral-dispatch 不能卸；④`foreign` 的只能显示、不能动；⑤真写用户主目录下的文件，测试只用临时 HOME；⑥`CLAUDE_CONFIG_DIR` 两边不一致；⑦已开着的 agent 要重开才看到新技能；⑧`~/.agents/skills` 不只 Codex 读，叫“Codex”不完全准。

### 用户 10-14 说的最终目的，和主控查的 herdr
- 用户：“其实我的最终目的是要想herdr的settings里面的一个agent适配的功能一样。如果系统中有其他的agent，但是还没集成到herdr中的时候，可以一键适配，你可以查一下herdr的这个功能。之后咱们继续讨论”
- herdr（本机 0.9.4，`herdr integration status|install|uninstall`；文档 `docs/.../integrations.mdx`、`agents.mdx`、`configuration.mdx`，v0.9.4）：
  - 设置里有 integrations 标签，给 PATH 上找到的 agent 推荐、一键装；首次启动的引导结束后直接打开这个标签。命令行同样能装、卸、看状态。
  - 内置 18 家（pi、omp、claude、codex、copilot、devin、droid、kimi、opencode、kilo、hermes、qodercli、qwen、cursor、mastracode、antigravity-cli、grok、letta）。每家的适配代码都在 herdr 里，“一键”只是把它写进那家自己的配置目录：钩子脚本或插件，加上在那家的 settings／hooks.json／config.toml 里登记（只动自己那几条，原子写）。带版本号，状态是 current／outdated（v8 < v9）／not installed；卸载只删自己加的。配置目录不存在就不装。
  - 状态有两层：没装适配也能靠读屏幕（每家一份规则）认出 idle／working／blocked；装了适配后用 agent 自己报的状态，并能在重启后接回同一个会话。antigravity-cli（agy）的适配只报会话，状态仍靠读屏幕；它的钩子登记在 `~/.gemini/config/hooks.json` 的 `herdr` 一块（事件 `PreInvocation`）。
- 和 corral 的不同：corral 不往 agent 的配置目录里装东西，四家都是每次 `corral start` 时用命令行参数临时接上（claude `--settings`、codex `-c hooks.…`、pi／omp `--extension`）；不认识的种类没有状态，也没有读屏幕的后备。所以 paddock 要做 herdr 那样的“一键适配”，适配本身（每家怎么接、装什么、状态）得在 corral 里，paddock 只做设置页。
- 用户 10-14 看过主控的汇报：“我理解你的意思了，其实herdr也是接口写好的。只支持这么多，然后探测到用户有agent在他的支持范围内，但是还没集成到herdr中的时候，就可以一键配置了”
- 用户 10-14：“我的理解也是这样，但是ranch的改动会有多大？这是最核心的repo。”主控看了 ranch（只读）：corral 共约 4,300 行；各家事件先翻成同一套事件名（SessionStart、UserPromptSubmit、PreToolUse、Stop……），判断状态的 `events.rs` 是共用的；按种类写死的只有三个文件：`hooks.rs`（`known` 名单、启动时怎么接上、怎么让它退出）、`events/raw.rs`（pi／omp 的原始事件翻译）、`skills.rs`（技能装到哪）。钩子只在 `CORRAL_EVENTS` 设了时才干活，长期装在 agent 配置目录里也不影响平时直接开的 agent。主控的估计和分步见对话（分三步：只读名单 → 调研 agy → 安装命令加第一家）。
- 用户 10-14：“等一下，我记得我之前就说过，各个agent是按照这插件的方式接入到ranch的（代码级别的插件类型）。”主控查（只读）：原来的 Python 版 corral（`../corral`）确实是这个结构——`src/corral/agents/` 下 `base.py` 加每家一个模块（claude、codex、pi、omp），在 `__init__.py` 的 `ADAPTERS` 里登记，每家写明种类、钩子文件、启动时怎么接、怎么退出；它的 ROADMAP 把“适配 pi／amp／omp”各当一件（R4、R5、R7）。搬到 Rust（Saddle `corral-core` → ranch）后这层被摊平了：每家的代码散成 `hooks.rs`、`events/raw.rs`、`skills.rs` 里的 `match` 分支。用户说过的原话主控在 paddock、ranch、corral 的文档里没找到。
- 同时查到的约束：corral DESIGN 第 14 节“不动全局配置”——钩子只随 corral 启动的那个 agent 生效，唯一例外是用户确认后的 `install-skills`；ranch 的两份设计文档也写了“不改用户全局配置”。amp 就是卡在这条上没接（只能从全局目录加载插件），ROADMAP R5 写了出路：“人决定接受一个例外：像 install-skills 一样经人确认后装一个全局插件，只在检测到 corral 的环境变量时生效。这要先改 DESIGN 第 14 节”。herdr 式的“一键适配”正是往 agent 的全局配置里写，和这条冲突。
- 用户 10-14：“先恢复适配器结构，全局配置可以放宽。 加入按照这个计划走，会有多大的爆炸半径？”主控只读查了 ranch 里用到按种类代码的地方：`hooks::argv`（`pen.rs:751`，每次 start 拼命令）、`hooks::known`（`cli.rs:132` 读状态、`cli.rs:516` 判断能不能带 `--prompt`）、`hooks::quit_steps`（`cli.rs:649`，stop）、`hooks::run`（`lib.rs:68`，钩子写事件）、`hooks::helper`（`upgrade.rs:9`，升级时切换钩子程序的链接）、`raw::apply_raw`／`settle`（`events.rs:204-214`，读 pi／omp 的事件）、`skills::run`（`cli.rs:431`）。判断状态的 `events.rs` 主体、`pen.rs`（PTY）、登记、接入都不碰。运行中的 agent 的钩子走 `~/.corral/.runtime/helper` 链接，只有 `corral upgrade` 才切换，平时部署新版不影响它们写事件；但新版的 `ls`／`status` 要读它们写的事件。爆炸半径的分步分析见对话。
- 用户 10-14：“好，那就先在 ranch 立任务，重构适配器结构。你把需求发给他，让他主控做，不再委派，这是一个有难度的活，让他小心点。”主控 10-14 用 `corral send` 把需求交给 `ranch/main`（request_id aa7069d4-4f16-4069-b349-64687f5f09ea，送达、对方开始干活）：恢复一家一个适配器、行为逐字节不变（先写改前快照测试）、ranch 主控自己做不委派、部署照 ranch 规矩由用户跑；不加新 agent、不加 integration 命令、不改全局配置相关代码。本件以后要依赖 ranch 那件和之后的 `corral integration status --json`。
- 10-14 ranch/main 回报（用户转来）：R3「corral 恢复适配器结构」已合并、推送、收尾（ranch main `7b761e7`）：`crates/corral/src/agents/`，`mod.rs` 是 `Adapter` 接口和登记表 `ADAPTERS`，claude、codex、pi、omp 各一个文件；行为逐字节不变（快照测试），公开约定都没变，paddock 不用动。ranch DESIGN §7 记了结构和用户“全局配置可以放宽”的条件（R3 没写这部分代码）。用户已部署：`~/.local/bin/corral`、`ranch` 指向 `versions/7b761e7`，`corral upgrade --all` 把 6 个在跑的 agent 的 pen 原地换了；退回办法是把链接切回 `versions/d55defb`。下一步：可以给 ranch 写只读 `corral integration status --json` 的需求了（还没写）。
- 用户 10-14 问四家能不能也像 herdr 那样做，指出“pi和omp不支持技能？我记得这四家现在都一样的。”主控核实：pi 读 `~/.agents/skills`，omp 读 `.agents/skills`、`.claude/skills`、`~/.codex/skills`；corral 的 `--target codex` 实际装到共用的 `~/.agents/skills`，四家都能用技能。主控提议本件收窄为四家：每行找到没有（路径、版本）、状态集成 `Built in`（启动时挂上，无按钮）、技能 Install／Update（调现有 `install-skills`）；名单和每家的技能目录由 corral 给出。用户问了这条命令的用途后：“按你的建议，先给ranch提这条命令”。主控 10-14 用 `corral send` 交给 `ranch/main`（request_id ad74b40e-a20e-42bb-aad2-e242fc2bbd3b，送达，回执 `merged_with_draft: true`，对方开始起草 R4）：只读 `corral integration status --json`，每家报种类、找到没有、版本（可选）、接法、技能目录和装没装；`--target codex` 命名由 ranch 定；不做 install/uninstall、全局钩子、agy、amp、读屏兜底。本件等它合并、写进公开约定后再定范围开工。
- 10-14 ranch/main 回报（用户转来）：R4 已合并（`b0e3b6f`，收尾 `653c5b2`）、部署（`~/.local/bin/corral`、`ranch` 指向 `versions/653c5b2`，退回切 `7b761e7`）。约定在 ranch `docs/CONTRACT.md`「corral integration status（R4）」：命令没有 `--json`（corral 都只出一行 JSON，带了报 usage）；输出 `{"ok":true,"agents":[{kind, path, version, hooks, skills:[{target, path, status}]}]}`，按登记顺序；`path` 按调用者 PATH 找；`version` 是 `--version` 第一行、每家最多 3 秒、拿不到为 null；`hooks` 现在都是 `launch`，以后有别的值要能显示；`skills.status` 同 `install-skills --dry-run`（create／same／overwrite／foreign）；位置 claude＝`$CLAUDE_CONFIG_DIR` 或 `~/.claude`（Claude Code、omp 读），codex＝`~/.agents`（Codex、pi、omp 读）；只读。`--target codex` 没改名。主控本机跑过，四家路径、版本、技能都对。

## 完成记录

2026-10-14，paddock/main 自己做。
- 做了什么：
  - 新模块 `app/src/integrations.rs`（不依赖 GPUI）：读 `corral integration status`（agent、位置、corral 技能）、`ranch dispatch install-skills --dry-run`（corral-dispatch）、paddock 自己的技能空跑；算每行显示什么（`rows`）、按钮会写什么（`plan`：哪些技能装到哪个位置、哪些文件新建或替换、写进哪个技能目录、还有哪几家也读）；照计划跑 `corral install-skills --target <位置> --yes`、`ranch dispatch install-skills --target <位置> --yes`、paddock 自己装（`install`），一步失败就停、报“技能: 原因”。
  - `app/src/skills.rs`：位置改成按 `--target` 名字（claude、codex）挑；claude 的位置认 `CLAUDE_CONFIG_DIR`（在 HOME 外面时检查到它自己为止）；加 `skills::install(targets, write)` 给 Settings 用；命令行的帮助写上 `$CLAUDE_CONFIG_DIR`。
  - `app/src/settings_integrations.rs`（`settings_view.rs` 的子模块）：Agents 页 corral 一节下面的 Integrations：标题旁 Refresh／`Checking…`／`Installing…`，列名（窄于 520pt 时不画、Status 和 Skills 挪到名字下面），每行名字和版本、`Built in` 绿签、Skills 的 `Up to date`（绿勾）／`Install`／`Update` 按钮和黄字说明／`Not managed`／`—`，没找到的整行变淡；卡片下两行淡字；读失败一句红点错误。确认框盖在 Settings 窗口上（点暗处或 Cancel 关），写明文件、目录、共用的几家。装完用底部消息条报结果再重读。
  - `settings_view.rs`：切到 Agents 页时读；Agents 页加这一节；窗口上加确认框。
  - `docs/DESIGN.md`：§3 公开约定表加一行（`corral integration status`、两条 install-skills）；§13 加 P5-94 一条。
- 验证了什么：
  - 先写测试看它失败再实现：`skills.rs` 三个（`CLAUDE_CONFIG_DIR` 在 HOME 里、在 HOME 外，`targets` 只看指定位置），先看到断言失败（路径还是 `~/.claude`、条数 2）；`integrations.rs` 十六个（解析、出错、名单顺序、不认识的 hooks、共用位置一起 Update、全没装是 Install 不带说明、缺和变合在一句、foreign 不算、没有 ranch、计划只写需要的、两个位置都写，以及用假 corral／ranch 脚本读、ranch 找不到或出错、corral 找不到或旧版不认识命令、装的时候每步的命令行、一步失败就停），存根时失败、实现后通过；`settings_integrations.rs` 一个（英文列举、单复数）。
  - 用一个临时小程序（跑完删了）对本机真的 corral、ranch 只读读了一次：四家路径、版本对，`Built in`，技能都 `Up to date`，没有按钮，和 `corral integration status`、`ranch dispatch install-skills --dry-run` 的输出一致。没有在本机真装过技能。
  - `app/` 下 `cargo test --all-targets` 全过（库内 490 个，加各集成测试）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 通过。
  - 没起窗口、没截图；页面样子、窄窗口、确认框和真点一次 Update，留给用户实际看。
- 拿主意的地方：
  - 说明里每个技能各说一次（`corral-dispatch changed, paddock missing`），不合并同类。
  - 按钮在读或装的时候变淡、点不了；装的时候标题旁写 `Installing…`。
  - 确认框只能点 Cancel 或点暗处关，没接 Esc。
  - corral 的 `version` 原样显示（Claude Code 是 `2.1.299 (Claude Code)`），不去掉括号。
  - paddock 技能在 HOME 外的 `CLAUDE_CONFIG_DIR`：那个目录要已经存在，不然装的时候报错（不替用户建它）。
- 没做的事：卸载；agy、amp（P5-88）；`paddock install-skills` 命令行没加 `--target`（Settings 直接调函数）。

## 用户看过
- 10-14 重开看过：“我重开看了，不错。就是有一个疑问，pi为什么显示not found，是不是我的pi agent安装的地方不对？”主控查：pi 装在 `~/.bun/bin/pi` 没问题；bun 把 `~/.bun/bin` 加进 PATH 的那两行写在 `~/.zshrc` 里，而 paddock 从 Finder／Dock 启动时问的是登录 shell（`zsh -l -c`，不读 `.zshrc`），拿到的 PATH 里没有它，corral 按这个 PATH 找不到 pi。同样从 paddock 里开 pi 也会找不到。
