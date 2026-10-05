# 任务 P2-4：菜单栏、快捷键与 .app 打包

2026-10-05，paddock/main 自己做。
类型：功能变更
依据：DESIGN §11 第 4 项。用户 10-05 选定：快捷键照 macOS 常见约定；图标用猫的像素图；打好的 `paddock.app` 放 `~/Applications`。

## 要做的

1. **菜单栏**（GPUI 的菜单与动作）：

   | 菜单 | 项（快捷键） |
   | --- | --- |
   | paddock | About paddock、Quit（⌘Q） |
   | Shell | New Tab…（⌘T，弹出选择）、New Shell（⌘N）、Split Right（⌘D）、Split Down（⇧⌘D）、Split Left、Split Up、Close Pane（⌘W）、Close Tab（⇧⌘W） |
   | Edit | Copy（⌘C）、Paste（⌘V） |
   | View | Fold Agents、Sort Agents by Name（勾选状态跟随侧栏） |
   | Window | Minimize（⌘M）、Zoom、Next Tab（⇧⌘]）、Previous Tab（⇧⌘[）、Tab 1–9（⌘1…⌘9） |

   - 分屏的菜单项直接分在对应方向，再弹出选择打开什么（同 `Split ▾` 的第二步）。
   - 弹框打开时 Esc 关闭。
   - 现在终端窗格里自己处理的 ⌘C、⌘V、⌘Q 改走菜单动作，行为不变（⌘C 有选区才复制）。
   - New Shell 的规则同侧栏的 `＋ New shell`。
2. **`.app` 打包**：用 Rust 写一个打包工具（不用 Python），把 release 版 `paddock` 打成 `paddock.app`：`Info.plist`（名字、标识 `dev.paddock.app` 之类、版本、最低系统版本、高分屏）、可执行文件、图标；图标由 `cat-image.toml` 的站姿生成各尺寸 PNG，再用系统自带的 `iconutil` 合成 `.icns`。工具支持安装到 `~/Applications/paddock.app`（覆盖旧的）。不签名、不公证（DESIGN §7）。
3. **从程序坞启动也要好用**：从 Finder 或程序坞启动时，环境变量很少（`PATH` 不含 `~/.local/bin`，工作目录是 `/`）：
   - 启动时向用户的登录 shell 取一次 `PATH`（同类应用的常见做法），让 `corral`、`git` 和 shell 里的程序都能找到；取不到时保留原样，侧栏照常显示 corral 的错误说明。
   - 没给 `--cwd` 时，新 shell 的目录用用户主目录，而不是 `/`。
4. `app/README.md` 写明打包、安装和快捷键。

## 怎么算做完

- 菜单栏和上面的快捷键都在、都能用；截图菜单给用户看。
- 运行打包工具后，`~/Applications/paddock.app` 能从 Finder、Launchpad、程序坞启动，图标是猫；启动后侧栏能列出 agent、shell 在主目录打开。
- 现有测试照样通过；新增测试覆盖：快捷键与动作的对应、取登录 shell `PATH` 的解析、图标像素生成；`cargo clippy --all-targets -- -D warnings` 无警告。
- 键盘、鼠标的实际操作和从程序坞启动的手感，留给用户体验。

## 不要做

- 不签名、不公证、不做自动更新、不做 DMG。
- 不做设置页、布局保存、新建 agent。
- 不安装 Xcode 组件或别的工具（只用系统自带的 `iconutil`、`sips` 等）。
- 不往 `/Applications` 或系统目录写东西。
- 不改 Saddle 仓库。
