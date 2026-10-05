# 任务 M1：去掉 `ratatui`、`crossterm`

2026-10-05，paddock/main 自己做。
类型：重构（外部行为不变）
依据：DESIGN §4、§10。M0 迁入的代码还借用了这两个库的几个类型；本轮换成 paddock 自己的类型，删掉这两个依赖。界面本来就全是 GPUI 画的，不受影响。

## 现在借用了什么

| 借用的类型 | 在哪里用 |
| --- | --- |
| `ratatui::style::Color` | `preset.rs`（三套主题的界面色、`parse_color`）、`theme.rs`（解析成具体颜色、`[colors]` 覆盖）、`sidebar.rs`（按名取色） |
| `crossterm::event::{KeyCode, KeyModifiers, KeyEvent}` | `keys.rs` 把 GPUI 按键转成它，再交给 `input::encode_key` |
| `crossterm::event::{MouseEvent, MouseEventKind, MouseButton}`、`ratatui::layout::Rect` | `view.rs` 组装鼠标事件，交给 `input::encode_mouse` |

## 要做的

1. **颜色**：在 `preset.rs` 定义 paddock 自己的 `Color`，取值范围和现在用到的一致：`Reset`、16 个具名色、`Indexed(u8)`、`Rgb(u8, u8, u8)`。`preset.rs`、`theme.rs`、`sidebar.rs` 改用它。配置里颜色的写法和报错文字不变。
2. **按键**：在 `input.rs` 定义 paddock 自己的按键和修饰键类型，覆盖 `keys.rs` 现在会产生的那些键（字符、回车、Tab/Shift-Tab、退格、Esc、方向键、Home/End、PgUp/PgDn、Insert/Delete、F1–F12；Shift、Alt、Ctrl）。`encode_key` 改收这些类型，输出的字节不变；`keys.rs` 直接从 GPUI 按键转成它们。
3. **鼠标**：在 `input.rs` 定义 paddock 自己的鼠标事件类型（按下、松开、拖动、移动、滚轮四个方向；左、中、右键；修饰键；所在格子），`encode_mouse` 改收它和终端的行列数，输出的字节不变；`view.rs` 改为组装这个类型。
4. 从 `app/Cargo.toml` 删去 `ratatui`、`crossterm`，更新锁文件。
5. 迁入文件开头的来源说明补一句“M1 起不再用 ratatui/crossterm 类型”；DESIGN §4、§10 和 `app/README.md` 同步。

## 怎么算做完

- `app/Cargo.toml`、`Cargo.lock` 里不再有 `ratatui`、`crossterm`；其余包的版本不变（锁文件只少不多）。
- 外部行为不变：按键、鼠标、粘贴发给终端的字节和现在一样；主题、`[colors]` 覆盖、报错文字和现在一样。
- 现有 72 项测试照样通过。编码相关的测试（`tests/input.rs`、`keys.rs` 里的测试）只改构造输入的写法，期望的字节一个不改。`cargo clippy --all-targets -- -D warnings` 无警告。
- 启动一次，只截本窗口，确认显示正常（截图不入库）。键盘、鼠标的实际操作留给用户体验。

## 不要做

- 不改按键、鼠标、颜色的行为，也不借机补新功能（比如右键、中键上报、新快捷键）。
- 不改迁入代码中与这两个库无关的部分。
- 不改 Saddle 仓库。

## 完成记录

2026-10-05，paddock/main。

- **做了什么**：`input.rs` 定义 paddock 自己的 `KeyCode`、`Modifiers`、`KeyEvent`、`MouseButton`、`MouseEventKind`、`MouseEvent`、`Area`，名字和取值与原 crossterm/ratatui 类型一致；`encode_key`、`encode_mouse` 改收它们。`keys.rs` 直接从 GPUI 按键转成 paddock 的按键，`view.rs` 组装 paddock 的鼠标事件。`preset.rs` 定义自己的 `Color`（`Reset`、16 个具名色、`Indexed`、`Rgb`），`theme.rs`、`sidebar.rs` 改用它。删去 `ratatui`、`crossterm`：锁文件少了 68 个包（含 termwiz、wezterm 系列），没有新增或升级。DESIGN 同步更新；顺带改正了 `theme.rs`、`sidebar.rs` 开头两处仍说“来自 Saddle 库”的注释。
- **验证了什么**：72 项测试全部通过；`tests/input.rs` 只改了导入和 `Rect::new` → `Area::new`，期望字节一个未改；`keys.rs` 的测试经 GPUI 按键走完整路径，未改。clippy 无警告（只剩 GPUI 依赖 `block v0.1.6` 的上游提示），`cargo fmt --check` 通过。release 构建启动一次，只截本窗口显示合成样例：颜色、侧栏正常；截图已删除。
- **拿主意的地方**：
  - `encode_mouse` 保留“区域”参数（`Area`，代替 ratatui 的 `Rect`），没有按任务书写的改成只收行列数：这样迁入的鼠标测试（区域带偏移、区域外的点不上报）原样有效。paddock 自己总是传 `(0, 0, 列, 行)`。
  - 去掉了 `encode_key` 对“按键松开”的判断：paddock 的按键只来自 GPUI 的按下事件，原判断在 paddock 里从未生效。
- **没做的事**：键盘、鼠标的实际操作没有测，留给用户体验；未改任何按键、鼠标、颜色行为。
