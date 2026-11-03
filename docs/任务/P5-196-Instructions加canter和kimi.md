# 任务：Settings › Instructions 加 canter 和 kimi；kimi 在 paddock 里显示找不到

2026-11-03，paddock/main 自己做。
路由：轻档自做，不问路由（主控判断：两家各加一行名单和路径、一句说明，改动点能一条条列出来）
类型：功能变更
依据：P5-195 验收时用户看到 Settings 里几处没有 canter、kimi 显示 Not found，要求记成一件；用户答了 Instructions 加 canter 和 kimi，kimi 的 PATH 由主控替用户挪。canter 的暂停、沙箱按能力显示和 Integrations 的 canter 一行在 P5-197（等 ranch R56a），Dispatch 也记在那里。
执行：主控
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。

## 用户原话
- 11-03 看 P5-195 时：「settings里面，agents我看不到，而且kimi显示找不到」「另外settings中dispatch也没有canter」「另外isettings中的instructs也没有canter。上面的问题还有kimi这种。都记录到一个新的任务文件中。」
- 主控问：Instructions 加 canter 吗；kimi 也一起加吗；kimi 找不到是挪 `.zshrc` 里那行到 `.zprofile`、改交互式 shell 取 PATH，还是加说明。用户：「1、2 都加，kimi你帮我挪」。

## 查到的
- Instructions 的名单是 `instructions.rs` 里写死的六家，P5-179 时用户没要 kimi 的说明文件。
- canter 的全局说明文件：canter `docs/DESIGN.md` 第 20 条，`$CANTER_HOME/AGENTS.md`，没设就是 `~/.canter/AGENTS.md`；不再读 `~/.agents/AGENTS.md`。
- kimi 的全局说明文件：本机装的 kimi 2.4.1 自带说明写 `$KIMI_CODE_HOME/AGENTS.md`，没设就是 `~/.kimi-code/AGENTS.md`，另外还读第一份可读、非空的 `~/.agents/AGENTS.md`。（kimi 官网的定制页写的是 `~/.kimi/AGENTS.md`，像是旧版 kimi-cli 的布局，以装着的版本为准。）
- kimi 显示 Not found：kimi 把 `~/.kimi-code/bin` 加进 PATH 的那行写在 `~/.zshrc`；paddock 用 `$SHELL -l -c` 取 PATH（`launch.rs`），不读 `.zshrc`。

## 要做的
1. Instructions 名单在 agy 后加 kimi、canter（和 New Agent 的顺序一样），路径照上面算（环境变量设了且不为空就用它指的目录）。
2. kimi 的卡片下加一句它还读 `~/.agents/AGENTS.md`，和 omp、agy 的说明一样。
3. kimi 的 PATH（主控替用户做，不改代码）：把 `~/.zshrc` 里 `# kimi-code` 和下面那行 `export PATH=…` 挪到 `~/.zprofile` 末尾，两个文件先各留一份备份。
4. `docs/DESIGN.md` P5-119 的表补两行。

## 怎么算做完
- Settings › Instructions 有 canter、kimi，各自显示、编辑它的全局说明文件。
- 重开 paddock 后 Integrations 里 kimi 不再是 Not found。
- 验证：`tests/instructions.rs` 先改断言看到失败，再实现；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各一次。界面留给用户重开看。
