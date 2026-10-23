# 交接

本文件只记照着执行的事实：本机装的是哪一版、怎么重新打包和退回，机器和环境的长期事实，还没派的草稿，已知没修的问题。只在这些变了时更新。

- 会话经过、进度、待定的事、约定和经验由 cairn 记（接续摘要、项目的账、经验），开局自动注入；这里不重复。现在有没有进行中的活、worktree、测试 agent，去现场查（`git worktree list`、`corral ls`）。
- 设计和理由在 `docs/DESIGN.md`，每件活的经过在 `docs/任务/` 和「合并」「收尾」提交里。
- 2026-10-23 之前的版本（进度流水、「下一步」、「用户用到时顺带看」）在 git 历史里：`git log -p -- HANDOFF.md`。

## 本机安装

- 现装：`~/Applications/paddock.app` 是 P5-167 的版本（10-23 从分支装，代码和合并提交 `324919a` 相同），本机 Apple Development 证书签名。
- 重新打包安装：在 `app/` 下 `cargo build --release`，再 `cargo run --release --example bundle -- --install`（编译目录 `.target/main`）。默认选本机 Apple Development 身份，可用 `PADDOCK_SIGN_IDENTITY` 指定；退回 ad-hoc 时会提示屏幕录制授权会失效。release 很快结束时，核对产物时间晚于合并再装。
- 退回：不留旧版。在临时 worktree 检出要退回的提交，照上面打包安装（编译目录用这个 worktree 自己的子目录）。旧版读不了新配置：P5-90 之前不认 `[[machines]]`，P5-120 之前不认预设里的 `sandbox` 字段，退回前先从 `~/.config/paddock/config.toml` 去掉。
- paddock ctl：`~/.local/bin/paddock` 链到 app 里的程序，重新打包不用重建。技能正文改了要重跑 `paddock install-skills`（装在 `~/.claude/skills/paddock`、`~/.agents/skills/paddock`、`~/.gemini/config/skills/paddock`），开着的 agent 重开才看到。
- 本机配置 `~/.config/paddock/config.toml`：写着 `frame_opacity = 2`（默认已是 0，Settings 点 `Reset to 0%` 才跟默认）；有一台 `[[machines]]`（见下面 5090）。
- 种类真图标在 `~/.config/paddock/icons/`（来源见同目录 `SOURCES.txt`），不进仓库；没有时退回仓库里的单色剪影。

## 依赖的外部命令

- `corral`、`ranch`、`fence`、`pony` 由 ranch 部署，现装哪一版、怎么退回看 ranch 的 `HANDOFF.md`（或 `corral --version`）。paddock 只调它们的公开命令；部署由用户在普通终端里跑。
- cairn：本仓库已 `adopt`。Claude Code（`~/.claude/settings.json`）和 Codex（`~/.codex/hooks.json`；直接启动的 Codex 要到 `/hooks` 里信任）都装了 hook。paddock 调的是 cairn DESIGN §7.1 的公开命令：`status --json`、`show [<ID>] --json`、`list --json`、`adopt`。0.7.0 的数据库是版本 5，0.6.x 读不了，退回要换回备份。退出：`cairn unadopt`；卸掉：`cairn uninstall --agent claude|codex`。

## 5090（用户的 Linux 实测机，主机名、地址不进仓库）

- 本机配置里的 `[[machines]]` 名字是 `5090`，ssh 别名在本机配置里。
- 远端 corral 是 `$HOME/.local/bin/corral`（ranch 部署）；非交互 ssh 的 PATH 里没有，要用绝对路径。在远端开 pi 要经远端 `zsh -l` 跑 `corral start`（pi 在 nvm 下）。
- 那台机器上只用 pi、omp，不跑 Claude Code、Codex，pi、omp 也不委派活（暂时的，放开以用户明说为准）。

## 仓库

- `origin` = `github.com/firegnu/paddock`（public）。推送前跑 gitleaks，截图、主机名、私人数据不进提交。
- 仓库顶层 `.fence.toml` 放行 `paddock-worktrees/.target`，套墙的 agent 不用再带 `CARGO_TARGET_DIR`。
- 分支 `p5-131b-kept`（agy 落选的 P5-131b）按用户 10-17 定留着，不删。

## 主控操作要记得

- 自己做的小活：建 worktree 后 `cp -cR ../paddock-worktrees/.target/main ../paddock-worktrees/.target/<分支>`（APFS 克隆）给它一份编译目录，收尾一并删掉。
- `corral start --unique` 会给名字加 `-1`，后续用返回的名字。新 worktree 第一次开 Codex 会卡在“是否信任这个目录”，要用户去点。
- `send --after` 的第一个参数是收件人（`corral send "$CORRAL_NAME" … --after <对方>`），没有撤销提醒的命令。提醒可能在合并之后才到：`corral status` 的 `state_started` 没变就是旧提醒。
- 派出去的 agent 停在提问对话框（`blocked`、`last_tool` 是 AskUserQuestion）：`corral read` 看问题，`corral keys <名字> "text:…"` 再 `corral keys <名字> enter` 把回答送进去（`esc` 关掉对话框后 `send` 仍会被退回 7），之后重新挂提醒。
- 关 agent 前确认它 idle、`attached` 为 0（paddock 窗口里显示着的会是 1）。
- 起测试窗口用临时 HOME 和假 corral；临时 HOME 不隔离 corral，要单独设 `CORRAL_HOME`，放 `/tmp` 下的短目录（socket 路径有长度上限）。

## 还没派的草稿（主仓库工作区里未提交，看板显示 DRAFT，提交时按路径 `git add`）

- `docs/任务/P5-54-选中内容送给agent.md`（优先：低）：Browser 选中元素、Changes 选中 diff 写上要怎么改后送给 agent；主控提议拆成 P5-54a Changes、P5-54b Browser，用户还没表态。
- `docs/任务/P5-111-调研OSC7501agent状态标准.md`：范围已定 5 条，派出后用户叫停（“有风险，而且收益不行”），退回草稿。
- `docs/任务/P5-115-用corral和cairn的仓库规矩统一.md`：范围、做法、验收都没定。
- `docs/任务/P5-146-把终端拆出去做自己的终端.md`（优先：低）：长期想法。
- `docs/任务/P5-149-终端按命令分段.md`（优先：低）：不排期。

## 已知没修的（不主动追，碰到再看）

- 强制退出 paddock、点 Dock 上多出来的 paddock／`exec` 图标，会把从 paddock 里开的 agent 一起结束（P5-52，同一个 coalition）；⌘Q 没事。从程序坞菜单“退出”或注销时不问运行中的 shell（GPUI 拦不住）。暂停 agent 时 Claude Code 的 `caffeinate` 断言仍在，Mac 不会睡。
- 收件箱回复以 `-` 开头的送不出去（P5-87，要 ranch 把 `corral send … --` 写进公开约定）；agent 输入框里有没提交的字时，回复会接在后面一起提交。
- 远程（P5-90）：断网、断线、重连没测过（不做断网测试）；退出时正在跑的远程查询可能多留最多 15 秒；只读条靠 `attached` 总数判断，多个客户端时可能过早消失。
- Settings：编辑时直接关窗口，改动不提示就丢；编辑框不自动滚到光标处（P5-119）。Dispatch 换选别的格子时没保存的改动直接丢（P5-161）。
- 不在前台去色（P5-162）不管 Browser 网页、agent 品牌图标、New Agent／About 小窗。
- 标签（P5-165）：终端信号的琥珀内光在标签和面板交界处没让开；从右边那块放大的几帧里标签会短暂画回胶囊。
- 终端：进度条和响铃让输出多过一遍 VTE 解析（P5-148b）；浮层淡出约 84 ms 内仍接点击；同步输出超时、连字、按窗格字号、终端图片和着色器用户 10-20 定不做（P5-143）。
- 动效：卡片滑动时分组标题和托盘直接跳（P5-154）；Loop 里 Now 的转圈不跟“减弱动态效果”（P5-158，用户：“先这样”）。
- GPUI 中文“最多几行”截断会估窄：Cairn 摘要已改（P5-137a），看板标题、收件箱可能也有。
- P5-131a：格子图存失败不重试；菜单两句提示只显示前一句；存失败后再选已在列表里的仓库不重试保存。
- 零碎：`cairn_view.rs` 的 `styled` 和 `kanban_view.rs` 里那份重复；右侧栏最窄 426 嫌宽（P5-55）；Cairn 记录点开不自动滚到它、行没有悬停底色（P5-79）；名字签在查找条开着时可能盖住它、窄窗格里右端被裁（P5-77）；任务弹框正文写到框底不滚、开着换主题不变色（P5-60b）；New Agent 弹出框不能用键盘上下选（P5-37）；看板切仓库的菜单没有滚动、Kept 里暂时不在的目录会被去掉、列表存不进去不提示（P5-85）；Drop task 的原因输入框全看板只有一个（P5-86）；同名仓库上级目录缩短后可能一样（P5-105）；`paddock ctl open --prompt … -- amp` 会被 corral 拒（P5-98）；卡片悬停不做渐变（P5-51）；DESIGN §13 的 P5-20 条重复了一遍。