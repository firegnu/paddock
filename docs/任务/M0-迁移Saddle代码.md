# 任务 M0：把用到的 Saddle 代码迁入 paddock，去掉 `saddle` 依赖

2026-10-05，paddock/main 自己做（用户 10-05 决定不再委派）。
类型：重构（外部行为不变）
依据：DESIGN §3、§10。本轮只搬代码、去掉 `saddle` 依赖；`ratatui`、`crossterm` 照原样保留，下一步 M1 再去掉。

## 范围

把 Saddle 提交 `df1c727` 中 paddock 实际用到的代码迁入 `app/src/`，`app/Cargo.toml` 删去 `saddle` 依赖。

| Saddle 文件 | 迁到 | 迁哪些 |
| --- | --- | --- |
| `pty.rs` | `app/src/pty.rs` | 全部 |
| `terminal.rs` | `app/src/terminal.rs` | `Size`、`Screen` 的解析和终端查询应答；不迁 `render`、`color`（写 ratatui 缓冲区）和 `history` 字段 |
| `viewer.rs` | `app/src/viewer.rs` | 全部，去掉只给 Saddle 布局保存用的 `remembered` 字段 |
| `input.rs` | `app/src/input.rs` | `encode_key`、`encode_mouse`、`encode_paste`；不迁 TUI 的焦点路由 `Focus`/`Route` |
| `corral.rs` | `app/src/corral.rs` | 全部 |
| `command.rs` | 并入 `app/src/corral.rs` | `corral` 用到的 `run`（带超时和取消的子进程调用） |
| `agents.rs` | `app/src/agents.rs` | `Panel` 的排序、分组、状态判断，以及 `absorb`、`group`、`Status`；不迁 git 摘要（`git` 字段、`absorb_git`）和只服务 TUI 键盘操作的字段与方法 |
| `theme.rs` | `app/src/preset.rs` | Dune、Tide、Lagoon 三套色值、`Theme` 结构、`named_mut`、`parse_color`；不迁 Terminal 预置和 ratatui 边框样式；`theme = "terminal"` 仍按现在的报错提示 |

- 每个迁入文件开头注明：来自 Saddle 提交 `df1c727` 的哪个文件，做了哪些删减。
- 调用处从 `saddle::…` 改成 `crate::…`。
- Saddle 中这些模块的测试一并迁入（`src/viewer.rs` 内的测试，`tests/` 下的 `pty.rs`、`terminal.rs`、`viewer.rs`、`input.rs`、`corral.rs`、`agents.rs`），只保留测迁入部分的用例。其中依赖 Python 假脚本（`tests/fixtures/corral.py`）的，按本仓库“不用 Python”的规矩改用 shell 假脚本；改不了的，在完成记录里列出来。
- 合并时一并改写 AGENTS.md 中引用 Saddle 的规矩（只通过 git 依赖引用、升级引用、交换类型的库同版本），改成 DESIGN §3 的关系；`app/README.md` 的依赖说明同步更新。

## 怎么算做完

- `app/Cargo.toml` 和 `Cargo.lock` 里不再有 `saddle`；其余第三方库版本不变（锁文件只少不多）。
- paddock 行为不变：shell、`--attach`、侧栏列表和点击接入、主题、字体配置照旧。
- 原有 53 项测试照样通过，迁入的测试也通过；`cargo clippy --all-targets -- -D warnings` 无警告。
- 启动一次，只截本窗口，确认侧栏和终端窗格显示正常（截图不入库）。

## 不要做

- 不去掉 `ratatui`、`crossterm`（留给 M1）。
- 不顺手改迁入代码的行为、命名或结构，除了上面列的删减和必要的路径改动。
- 不改 Saddle 仓库。
