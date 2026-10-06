# 任务：New Agent 开 agent 时从命令里认出模型和强度，记成 model、effort 标签

2026-10-06，paddock/main 交给 paddock/dev-model-label（Codex，常规：gpt-6-astra / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（0.91，置信 0.86）；交叉审查不要；改行为）
类型：功能变更
依据：本轮只让 New Agent 窗口拼出的 `corral start` 调用带上 `model`、`effort` 标签；不改卡片的显示规则，不改 corral/ranch，不管已经开着的 agent。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 为什么
用户 10-06 看侧栏卡片：“Model现在显示 -”。卡片的 MODEL 格只读 agent 的 `model`、`effort` 两个标签（`app/src/card.rs` `details`），两个都没有时写 “—”。派活工具开的 agent 会带这两个标签，但 New Agent 窗口开 agent 时只加 `role=…`，所以从 paddock 开的 agent 这一格总是 “—”。主控提议“New Agent 从命令里认出 `--model` 和 `--effort`，自动加上标签”，用户：“做吧”。

## 先读
- `AGENTS.md`「规矩」一节（编译目录、测试不依赖真实 agent）。
- `app/src/new_agent.rs`：`Form::args`、`Form::preview` 和文件末尾的测试。
- `app/src/card.rs` 的 `details` 里 Model 那一格怎么读标签（只读，不改）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-21-model-label`，分支 `p5-21-model-label`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-21-model-label`。
- 只改 `app/src/new_agent.rs`。

## 要做的
`Form::args` 在 `--label role=…` 之后，按命令里认出的值再加 `--label model=<模型>`、`--label effort=<强度>`；认不出的那个就不加，两个都认不出时参数和现在完全一样。“Will run” 预览走的是同一份参数，自然跟着变。

按命令第一个词（取路径最后一段）认程序，不按 Codex / Claude 开关认（命令可以手改）：

- `claude`：`--model <值>` 或 `--model=<值>` 是模型；`--effort <值>` 或 `--effort=<值>` 是强度。
- `codex`：`-m <值>`、`--model <值>`、`--model=<值>` 是模型；`-c` 或 `--config` 后面的 `model_reasoning_effort=<值>` 是强度，值两边的引号去掉（命令 `-c 'model_reasoning_effort="high"'` 记成 `effort=high`）。
- 别的程序不认，不加标签。
- 同一个参数出现多次，以最后一个为准。值照原样写进标签（例如 `opus[1m]`），空值不加。

例子：命令 `claude --model 'opus[1m]' --effort high` 拼出 `… --label role=controller --label model=opus[1m] --label effort=high --prompt … -- claude --model opus[1m] --effort high`；命令 `codex --yolo -m gpt-6-astra -c 'model_reasoning_effort="xhigh"'` 带 `model=gpt-6-astra`、`effort=xhigh`。

## 怎么算做完
- 用户原话：“Model现在显示 -”；用户同意的做法：“New Agent 从命令里认出 `--model` 和 `--effort`，自动加上标签”。
- 上面「要做的」里两种程序的写法都能认出，没写模型强度的命令（包括默认的 `codex --yolo`、`claude`）拼出的参数和现在一样，现有测试不改期望。
- 验证只做这些：一条针对这件活的测试（覆盖 claude 和 codex 各一个带模型强度的命令），先确认它在现在的代码上失败，再改到通过；在 `app/` 下跑一次 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改卡片、侧栏的显示规则和文字，不改 New Agent 窗口的样子和字段。
- 不改 `corral.rs`、ranch、corral；不给已经开着的 agent 补标签。
- 不加依赖，不改 `Cargo.lock`。
- 不起窗口、不截图；不要用 `osascript`、System Events 等任何方式模拟按键或鼠标。
- 不要真的 `corral start`：测试只检查拼出的参数。`corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的，停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-21-model-label` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
