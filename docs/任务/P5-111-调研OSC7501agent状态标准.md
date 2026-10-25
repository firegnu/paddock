# 任务：调研 Ghostty 作者提出的 OSC 7501（agent 状态的标准）

2026-10-15 paddock/main 记。10-20 看板 Loop 送来开工，用户把范围定为下面主控提议的 5 条（“照这 5 条做”）；派给 Claude Code 后没出结果，用户又叫停：“这个任务暂时不做了。我觉得有风险，而且收益不行。先退到queue中”。worktree、分支已清，agent 已停，等用户再排。
依据：用户 10-15：“加一个draft task： 调研ghostty作者提出的OSC 7501， 这是agent状态的标准。”

## 现状（主控 10-15 凭已知写的，开工前再核对）
- paddock 现在拿 agent 状态只走 corral 的公开命令（`corral ls`／`status` 的 `state`、`last_tool` 等，DESIGN §3），不解析终端里的转义序列。
- 本仓库里没有提到 OSC 7501 的地方；主控还没查过这份提案，下面要问的都等调研回答。

## 调研要回答的（主控提议，用户 10-20 定了照这 5 条）
1. 提案本身：出处（作者、链接、日期、现在的状态是草案还是已定），序列的格式，能表达哪些状态，谁来发（agent 自己、包它的程序），终端怎么收。
2. 谁已经在用：Ghostty 自己、Claude Code、Codex、pi、omp、amp、agy 这些 agent 有没有发；别的终端或多路复用器（Zed、tmux、herdr 等）有没有收。
3. 和 paddock 的关系：paddock 内置终端收到后能做什么（窗格标题、侧栏状态、收件箱）；和 corral 给的状态是互补、重复还是会打架；corral attach 中间转不转这个序列。
4. 和 ranch／corral 的关系：corral 要不要发或转它，这类改动归 ranch，只写成建议，不在这件里改。
5. 许可：参考的实现若是 GPL（如 Zed 的应用层），只读思路不抄代码。

## 产出（主控提议）
- 一份调研结论写在本文件末尾（或 `docs/调研/` 下一份），带出处链接；最后给“做不做、先做哪一步”的建议，由用户定。

## 不做
- 这件只调研，不改代码，不改 ranch、corral。

## 调研结论（主控 10-25，用户看过后说“继续挂着这个任务”）

只读调研，没改代码，没找 ranch。Claude Code 只看了公开的更新日志和文档，没看源码；Codex、pi、omp 看了源码。

### 1. 提案本身
- Mitchell Hashimoto 10-06 发布，名为 Program Status Protocol，现为 rev 0.4（10-10），仍是草案。规范挂在他公司产品 Rex 的文档站，没有标准组织，没写许可：https://www.superlogical.com/rex/docs/build/program-status ；博客 https://mitchellh.com/writing/program-status-osc7501
- 格式：`ESC ] 7501 ; key=value:key=value ST`。`state` 必填：idle／working／blocked／done／error，另有 `clear`（清掉记录）。`kind` 只在 blocked 时有意义：permission／question／auth。可选键：`progress`（0–100）、`app`、`id`（用 `/` 分层，可同时有多条记录）、`title`、`msg`（这两个是 base64）。
- 每条报告整条替换对应记录。进程退出或新提示符出现（OSC 133 A）时，working、blocked 作废，done、error 保留，由终端决定什么时候不再显示。
- 探测：程序发 `OSC 7501;?`，支持的终端回 `?`；程序紧跟着发 DA1，先收到 DA1 的回答就说明不支持。
- 规范没讲多路复用器、转发和重发。

### 2. 谁在用（核到 10-25）
- 发报告的 agent，本机现装版本都已支持：
  - Claude Code 2.1.310（2.1.295 起）：先探测，后台会话不发。`CLAUDE_CODE_DISABLE_TERMINAL_TITLE=1` 关掉探测和报告。终端回答支持后，它不再在标题里放状态符号。
  - Codex 0.164.0（0.163 起；done／error 和 kind 从 0.164 起）：不探测，只要输出是终端就发，没有开关。代码在 `codex-rs/tui/src/terminal_program_status.rs`。
  - pi 1.4.0（1.1.0 起）：先探测；`PI_PROGRAM_STATUS=1` 不探测直接发，`=0` 关掉。
  - omp 18.12.6（18.8.7 起，自己的实现）：不探测；设置项 `terminal.programStatus` 可关；不发 auth。
  - agy 1.4.7（1.4.0 起）：先探测；`AGY_CLI_OSC7501=off|on`。
  - amp：ranch R50 录到它启动时发探测，发哪些状态没查到。
  - Kimi：没有支持，只有一个没人回的请求 MoonshotAI/kimi-code#4172。
- 收报告的终端和多路复用器：
  - 已发布：iTerm2 3.7.5，cmux（侧栏状态行）。
  - 只在开发版：kitty 合在 master，排在未发布的 0.50；Ghostty 只在底层库 libghostty-vt，应用本身忽略。
  - 拒绝：tmux #5713，作者说为时过早，tmux 会丢掉这个序列。
  - 没有：herdr 不打算做（#5069 转进讨论区）；Zed、WezTerm 没找到 issue 或 PR。

### 3. 和 paddock 的关系
- paddock 现在不认 7501，也没人回答探测。内置终端会回答 DA1，所以 Claude Code、pi、agy 都判断“不支持”，不发报告；Codex、omp 照发，被悄悄丢掉。
- 接收的位置是现成的：`app/src/terminal.rs` 里的旁路解析器（`Side`）已经在读 OSC 9;4 和 133。
- 只在 paddock 做有缺口：
  - corral 的 pen 只在有客户端连着时把原始输出转出去。
  - 重新连上时只补发终端模式（`terminal.rs` 的 `replay`），不补发历史输出。
  - 没连着时也没人回答探测。
  - 所以后台开的 agent 状态会丢；重新连上后要等下一次换状态才看得到。
- 和 corral 现有状态是补位关系，但会重叠。corral 靠每家的钩子，钩子报不出“在等用户”的再靠读屏幕兜底。7501 补的正是“在等你、等什么”这块；如果 paddock 同时显示两个来源，可能说法打架。

### 4. 和 ranch／corral 的关系（只是建议，没发出去）
- 最合适的接收方是 corral 的 pen：它一直看着全部输出，可以回答探测、记住最新的一条记录，放进 `corral status`。paddock 照旧只走公开命令。
- 要注意：pen 和 paddock 不能都回答探测。

### 5. 许可
- 规范没写许可。参考实现里 Codex 是 Apache-2.0；Zed 没有相关实现。

### 建议（待用户定）
1. 交给 ranch 评估：在 pen 里接收 7501，在 `corral status` 加一个状态字段。
2. 之后 paddock 用这个字段，在 Needs you 上标出是在等权限、等回答还是等登录。
3. 可选：paddock 普通 shell 窗格里直接跑的程序，由 paddock 自己认 7501。

风险：规范才出三周、一人主导、tmux 拒绝；好处是主流几家 agent 都已经在发。

### 出处
- 二手汇总（10-15）：https://stackness.dev/blog/osc-7501-program-status-terminal-protocol-for-coding-agents
- Claude Code：https://github.com/anthropics/claude-code/blob/main/CHANGELOG.md ，https://code.claude.com/docs/en/terminal-config.md
- Codex：https://github.com/openai/codex/blob/rust-v0.164.0/codex-rs/tui/src/terminal_program_status.rs ，https://github.com/openai/codex/pull/52725 ，https://github.com/openai/codex/pull/54114
- pi：https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/terminal-setup.md
- omp：https://github.com/can1357/oh-my-pi/blob/main/packages/coding-agent/src/utils/run-status.ts
- agy：https://github.com/google-antigravity/antigravity-cli/releases/tag/1.4.0
- Kimi：https://github.com/MoonshotAI/kimi-code/issues/4172
- Ghostty：https://github.com/ghostty-org/ghostty/pull/14560
- kitty：https://github.com/kovidgoyal/kitty/issues/10638
- tmux：https://github.com/tmux/tmux/issues/5713
- iTerm2：https://github.com/gnachman/iTerm2/blob/master/docs/notes-3.7.5.txt
- herdr：https://github.com/herdrdev/herdr/issues/5069
- cmux：https://github.com/manaflow-ai/cmux/blob/main/docs/program-status.md
- 本地依据：ranch `docs/调研/R50-corral回答终端颜色查询.md`（录到的探测字节），ranch `crates/corral/src/pen.rs`、`terminal.rs`（转发和补发），paddock `app/src/terminal.rs`（旁路解析器）
