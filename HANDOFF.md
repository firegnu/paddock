# 交接

只记长期状态：现在装的哪一版、下一步、悬着的事。每件活做了什么、怎么验的、用户怎么说，在 `docs/任务/<编号>…md`（完成记录、主控审查）和 git 的「合并」「收尾」提交里；设计和理由在 `docs/DESIGN.md`。10-09 起不再写按日期的流水账（之前的在 git 历史里，`git log -p HANDOFF.md`）。

## 现在在哪（2026-10-10）

- main 和 `origin/main`（`github.com/firegnu/paddock`，public）同步；没有进行中的活、没有 worktree，本地只有 main。工作区里两份未提交的草稿任务文件（P5-54、P5-78，看板 DRAFT；**有意留的，不要 `git add -A`**）。ranch 同样干净。corral 里除 `paddock/main`（本主控）之外都是用户的。
- 541 项测试通过，clippy、`cargo fmt --check` 干净。`~/Applications/paddock.app` 是合并 P5-55 之后的版本（10-10 装，本机 Apple Development 证书签名）；P5-55 右侧栏第四个标签 Cairn（cairn 的状态、下次会话会接到的记录全文；没采用的仓库确认后可 Adopt；右侧栏最窄从 320 变成 426），**待用户看**（主控没起窗口看过）；P5-77d 名字签不跟着光标、固定在输入框右上角（用户用了 P5-77c 后：“现在追随光标我觉得有点影响我。我想使用右边那个方案”），**待用户看**；P5-77c 名字签亮起时跟着光标、变大、滑出，打字期间一直亮（样稿 D，用户嫌 P5-77 的位置和效果不明显），用户 10-10 试过：“这个看起来永远都不会看漏了。哈哈哈”；P5-77 开始打字时输入框角上亮出名字签（样稿 B）、P5-77b 换了打字对象后的第一个字一定亮（用户反馈切换 agent 后该亮不亮），用户 10-10 试过：“codex-1现在可以了”；P5-76 分屏标签的图标叠成一摞卡片、前卡歪着，用户 10-10 看过：“看到了，做的真好”；P5-70～P5-75 用户都看过或试过，没问题。
- 重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录 `.target/main`）。打包默认选本机 Apple Development 身份，可用 `PADDOCK_SIGN_IDENTITY` 指定；退回 ad-hoc 时会提示授权会失效。
- **paddock ctl 已部署**：`~/.local/bin/paddock` 链到 app 里的程序，重新打包后不用重建；技能正文改了要重跑 `paddock install-skills`（装在 `~/.claude/skills/paddock`、`~/.agents/skills/paddock`），已开着的 agent 要重开才看到。
- 全局 `~/.local/bin/corral`、`ranch` 是 ranch `d55defb`（R2：派活和临时委派的 agent 带 `CAIRN_DISABLE=1`；支持 pause／resume）。
- **cairn 已全局装进 Claude Code**（`~/.claude/settings.json` 里 4 个 hook 和放行 `cairn save` 的规则，改前备份 `~/.claude/settings.json.bak-20261008T082855…`）；owlet 和 **paddock 都已 `adopt`**（paddock 10-10 起，P5-53：和全量 HANDOFF 并存试一两周，10-24 前后和用户定 HANDOFF 里“进度”那部分删不删；试点看新会话里 cairn 注入的停点、下一步、待用户决定准不准）。退回：`cairn unadopt`（记录留着）；卸掉：`cairn uninstall --agent claude`。**Codex 的 hook 10-10 也装了**（用户：“给codex也装上吧”；`~/.codex/hooks.json` 加了 4 处 `cairn hook codex`，改前备份 `~/.codex/hooks.json.bak-20261010T071731…`）：直接启动的 Codex 要用户到 `/hooks` 里信任（corral 启动的跳过），没在真实 Codex 会话里验证过触发；卸掉：`cairn uninstall --agent codex`。paddock 现在调用 `cairn status --json`、`cairn show --json`、`cairn adopt` 三条（DESIGN §3）；AGENTS.md 里“只通过 `corral`、`ranch` 命令打交道”那句还没把 cairn 写进去，要不要补问用户。**cairn 10-10 升到 0.2.0**（用户让主控直接在 cairn 仓库做的 F2，cairn 的 `docs/tasks/F2-*.md`、它的 HANDOFF 已更新）：`cairn status --json` 每家多了 `last_seen`（当前仓库里四种 hook 各自最近一次触发的时间），这三条命令的输出写成了 cairn DESIGN §7.1 的公开约定（只加不改）；数据库升到版本 2，升级前的备份在 `~/.local/state/cairn-backup-20261010-before-v2/`。
- 已合并：第一至第三阶段、迁移 M0–M3、P4-1～P4-3、P5-1～P5-77（P5-52 只查明原因、改法用户暂不定；P5-54 还是草稿）。
- 上下文：`AGENTS.md`（规矩，含「开发方式」里的看板约定）、`docs/DESIGN.md`（§13 是界面改版的全部决定和用户原话）、`docs/背景与决策记录.md`、`docs/设计稿/`、`docs/调研/`。

## 下一步

1. **P5-78** Cairn 标签状态行显示每家 hook 最近一次触发的时间（草稿，执行：主控；cairn 那一半已做完装好）：等用户看任务文件、答两处（只写“最近一次”还是四种事件分开列；`never` 下面加不加一行小字），答完主控自己做。
2. **P5-53 试点中**（10-10 已 adopt，HANDOFF 全量保留、规矩没改）：10-24 前后和用户回看，定 HANDOFF 瘦不瘦、“先读 HANDOFF”改不改。owlet 试点情况：HANDOFF 减到只留稳定背景，重开后能准确说出停点、已完成、下一步、待用户决定。确认某次注入了没有：只读查 `~/.local/state/cairn/cairn.db` 的 injections 表（`sqlite3 -readonly`）；日常看 `cairn list`／`cairn show`。
3. **P5-54** Browser 选中元素、Changes 选中 diff 写上要怎么改后送给 agent（优先：高）。主控提议（用户还没表态）：设计和交互样稿主控自己做；实现派 Claude Code 常规档，拆成 P5-54a Changes、P5-54b Browser（WKWebView 注入 JS 加 WKScriptMessageHandler，代码库里还没有），a 合并后再派 b；测试只用假 corral。下次从列出待定问题开始。
4. **P5-55 之后**（Cairn 标签第一版已合并，没排）：历史记录列表、点开单条记录要等 cairn 出 `cairn list --json`（给 cairn 主控提需求，用户定）；Unadopt、更正／撤回记录用户没要。
5. 以后：Changes 第二步（行上评论发给 agent、暂存、撤销，可能和 P5-54a 合并考虑）；Kanban 能拖来纠正（用户：排在后面）。

## 用户用到时顺带看（不用决定、不挡任何事）

主控没法替用户试的只有真实悬停、拖动、键盘、输入法：

- P5-55 Cairn 标签（主控没起窗口看过）：右侧栏拖到最窄（426）时四个标签和两个按钮是不是正好一行，界面字号调大后也看一眼；正文的分节、折行；面板开着时 `N uncollected` 基本是 0（`cairn show` 读的时候会顺手收取）；在一个没采用的仓库里看 `Adopt…` 和确认卡片（真点 `Adopt` 会采用那个仓库，`cairn unadopt` 撤回）；hook 没装、没装 cairn 这些状态平时碰不到。
- P5-77c、P5-77d 名字签（P5-77d 起两枚都在输入框右上角，主控没起窗口看过）：在右边够不够醒目（不够可加样稿 D 里“窗格的边同时亮一下”）；输入折到多行时落在哪（应在输入框上边框的右端，不压自己打的字）；Codex 里落在哪（找不到空行会跑到右下角）；实心强调色在各主题下刺不刺眼；打字期间一直亮、停 3 秒或回车后淡回的手感。
- P5-61 Agent shell（新建面板第三行；只支持 zsh、bash）；P5-59 agent 窗格标题行里会话题目（`corral attach` 不转标题的话改读 `corral status` 的 `title`）。
- P5-66 退出时只在 shell 有程序时才问（停在提示符 ⌘Q 直接退；跑着 `sleep 30` 或 `sleep 30 &` 时问）；P5-56 Settings 改了立即生效、Undo、“not installed”。
- 10-07 那批：拖缝调大小和双击；⌘Z／⇧⌘Z 和输入法；标签“+N”菜单；四套新主题（Rosé Pine 光标偏暗）；ctl 的 busy、`--focus`、`--attach`；格子图扫光、光晕、呼吸；一键暂停（会冻住包括主控在内的全部 agent）。
- Browser：P5-28c 照 `docs/任务/P5-28c-Browser网页策略与查找.md` 完成记录末尾 8 条清单（第 1 条本地地址最要紧）。Kanban 状态推得对不对、Needs you、Clear 的确认。

## 悬着

- **别强制退出 paddock、别点 Dock 上多出来的 paddock／`exec` 图标**：从 paddock 里开的 agent 都在 paddock 的 coalition 里，强制退出会把它们一起结束（10-09 出过一次，见 P5-52；主控建议测试实例用 Accessory 策略，用户“暂时不改了”）。正常 ⌘Q 没事。**主控不在 agent 会话里起测试窗口**（截图、实测），样子留给用户看。
- **auto 模式拦下主控改 `~/.local/bin`**：ranch 部署（切链接、`upgrade --all`、装技能）由主控打包、核对，用户在普通终端里跑。
- Saddle 用户基本不用了；Saddle 仓库里的 `t76-*` 分支、worktree 由 Saddle 主控处理。
- 从程序坞菜单“退出”或注销时由系统直接结束，不问运行中的 shell（GPUI 没有拦截）。Claude Code 带的 `caffeinate` 冻住后“不睡眠”断言仍在，暂停 agent 不等于让 Mac 能睡。
- 建议改未排：`cairn_view.rs` 里画 Markdown 行内样式的 `styled` 和 `kanban_view.rs` 里那份重复；右侧栏最窄 426 嫌宽的话要另想办法（比如窄的时候标签只留图标）（P5-55）；名字签在查找条（⌘F）开着且光标在最上面几行时会盖住查找条、窗格窄到放不下名字时右端被裁（P5-77）；任务弹框正文写到框底不自动滚到光标处、弹框开着换主题不变色（P5-60b）；New Agent 的弹出框不能用键盘上下选（P5-37）；DESIGN §13 的 P5-20 条重复了一遍；卡片悬停不做渐变（P5-51）。
- `docs/DESIGN.md` §7 其余待定：GPUI 依赖渠道、pre-1.0 是否接受、gpui-component 与首期是否只做 macOS、发布方式（只做了本机签名，不公证、不分发）。Metal 工具链不装，靠 `runtime_shaders`（P5-62 量过代价很小）。
- 主控做法备忘：
  - 自己做的小活：建 worktree 后 `cp -cR ../paddock-worktrees/.target/main ../paddock-worktrees/.target/<分支>`（APFS 克隆）给它一份编译目录，只做增量编译；收尾一并删掉。
  - 派活：“不要做”里写明不用 Python、命令里不用 `rm`、不用 `sh -c` 包长命令、前台命令别带等标准输入的东西；`corral start --unique` 会给名字加 `-1`，后续用返回的名字；新 worktree 第一次开 Codex 会卡在“是否信任这个目录”，要用户去点。
  - `send --after` 的第一个参数是**收件人**（提醒自己写 `corral send "$CORRAL_NAME" … --after <对方>`）；corral 没有撤销提醒的命令。提醒可能在合并之后才到：核对 `corral status` 的 `state_started` 没变就是旧提醒；收到提醒先看 `git log` 和 `corral read` 再动。
  - 派出去的 agent 用提问对话框停下来时（`corral status` 是 blocked、`last_tool` 是 AskUserQuestion）：`corral read` 看尾部的问题；`corral keys <名字> esc` 关掉对话框后 `corral send` 仍被退回 7，用 `corral keys <名字> "text:…"` 再 `corral keys <名字> enter` 把回答送进去；之后重新挂提醒。
  - 关 agent 前确认它 idle、`attached` 为 0（paddock 窗口里显示着的会是 1）；release 构建很快结束时，核对产物时间晚于合并再安装。
  - 测试截图用临时 HOME、假 corral、`--bounds` 和预写布局（现在先不起窗口，见上）；corral 的 socket 路径有长度上限，单独的 `CORRAL_HOME` 放 `/tmp` 下的短目录。
