# 任务：远程 New Agent 找不到 nvm 下的 pi；Directory 框里的 `~` 没画出来

依据：用户 10-17 试 P5-124，截图里 RTX-5090 只列出 Claude、Codex、omp：“因为那台机器上还有amp， pi”。主控只读查明：5090 的 `$SHELL` 是 zsh，nvm 写在 `~/.zshrc`，paddock 用的 `zsh -l -c`（登录、非交互）不读 `.zshrc`；`zsh -l -i -c` 四种都找得到（0.65 秒、没有多余输出）。截图里 Directory 框看着是空的，但光标在一个字符之后、Name 占位字是 `home/claude`，框里其实有 `~`、没画出来。用户选了“改成 -l -i”“一起修”；amp “先不列”。
执行：主控
优先：高

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-124b-login-shell`，分支 `p5-124b-login-shell`（已从 main 建好）。

## 要做的
1. 查种类和 `corral start` 都经远端的交互登录 shell：`"${SHELL:-/bin/sh}" -l -i -c`，标准输入不接终端。
2. 对话框打开时 Directory 框里的 `~` 画出来。

## 怎么算做完
- 用户原话：pi 在那台机器上，要列出来；选了“改成 -l -i”“一起修”。
- 先改测试：假登录 shell 照 5090 的样子，agent 的目录只在交互时读的 rc 里加进 PATH，确认现在找不到（红），再改到找得到（绿）。`~` 的显示能写测试就写，写不了在完成记录里写清原因和改了什么。`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check`。

## 完成记录

- 做了什么：`remote_start::login` 从 `exec "${SHELL:-/bin/sh}" -l -c '…'` 改成 `exec "${SHELL:-/bin/sh}" -l -i -c '…' </dev/null`（交互登录 shell，标准输入接空），查种类和 `corral start` 都走它；DESIGN §13 P5-124 那条跟着改。
- 第 2 条（`~` 没画出来）不是 bug：放大截图，`~` 正好被用户画的蓝色箭头盖住；光标在一个字符之后、Name 占位字 `home/claude` 都说明框里有 `~`。没改代码。
- 验证：先改集成测试的假登录 shell 照 5090 的样子（`-l` 读 profile、`-i` 才读 rc，agent 目录只在 rc 里加进 PATH），改代码前 `the_kinds_offered_are_those_the_login_shell_there_finds` 红（找到 `[]`，期望 `[Claude, Pi]`），改后绿；单元测试跟着改成 `-l -i -c` 和 `</dev/null`。主控在 5090 上只读跑了改后的原样查种类命令，答 claude、codex、pi、omp（没起任何 agent）。`app/` 下 `cargo test --all-targets` 全过（lib 584），clippy、fmt、`git diff --check` 干净。
- 拿主意的地方：标准输入接 `/dev/null`，免得交互 shell 的 rc 去读 ssh 的输入。
- 没做的事：amp 照用户“先不列”；自定义命令另开 P5-124c。
- 踩过的坑：主控本机 `cat` 被别名成 bat，管道里用 `cat -v` 报错，一度误以为是远端交互 shell 的输出。
