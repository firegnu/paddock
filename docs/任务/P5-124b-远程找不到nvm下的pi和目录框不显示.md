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
