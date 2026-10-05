# 任务 P3-1：Settings 设置页

2026-10-05，paddock/main 自己做。
类型：功能变更
依据：DESIGN §12 第 1 项。照 Saddle 的 Settings（`df1c727` 的 `src/settings.rs`、README “Settings”、`docs/UI主题设计.md`），信息和功能不少于 Saddle，外观用 GPUI 做得更好看。用户 10-05：不涉及 corral、遥测、Drover 这类共享运行时的都可以做；遥测总开关这次不放。

## 要做的

1. **打开方式**：菜单 paddock → Settings…（⌘,），在窗口里打开设置面板；左边是页面列表，右边是该页的设置项。
2. **页面与设置项**（照 Saddle，并加上 paddock 自己的设置）：
   - **General**：Sidebar width、Refresh interval（新配置键 `refresh_ms`，默认 1000，同 Saddle）、Mascot（开关）、Pet（clawd / cat / capybara）、Font、Fallback fonts、Font size、Line height。
   - **Colors**：Theme（Dune / Tide / Lagoon），下面是全部颜色，按 Saddle 的分组（Interface、Agents panel、Status、Agent types），再加 paddock 的终端配色一组（20 个 `terminal_*`）；每项有色块和取值，被覆盖的标 `custom`。
   - **Advanced**：corral command（新配置键 `corral`，默认 `corral`；命令行 `--corral` 仍可临时覆盖）。
   - 不做 Saddle 的 Diagnostics、Plugins、Updates 页，也不放 Telemetry recording（遥测，见 DESIGN §3）。
3. **编辑规则照 Saddle**：
   - 修改先是草稿；Save（⌘S）才写文件，Cancel（Esc）放弃；每项有 Default 恢复默认（Save 时照样写入）。
   - 换主题时把该主题的全部颜色载入草稿，并清掉颜色覆盖；之后再改的颜色标 `custom`，对它按 Default 就重新跟随主题。
   - 写错的值报错并指出是哪一项，草稿保留。
   - Save 只写改过的键，保留文件里的注释和其他内容；文件和目录不存在时创建。
   - 打开设置后，若配置文件在磁盘上被别处改过，Save 什么都不写，并提示：Keep my edits（重读文件，在其上保留草稿）或 Discard my edits（以文件为准）。
4. **生效**：主题、颜色、侧栏宽度、宠物开关和种类保存后立即生效；字体、刷新间隔、corral 命令标 `Restart required`，重启后生效（同 Saddle 对需重启项的做法）。
5. **文本输入**：GPUI 没有现成的输入框；参照 GPUI 自带的 `examples/input.rs`（Apache-2.0，与 GPUI 同许可，保留来源说明）做一个单行输入框，支持中文输入法、复制粘贴、选择。

## 怎么算做完

- 各页截图给用户看，外观认可；所有设置项都能改、能保存、能取消、能恢复默认。
- 新增测试覆盖：只写改过的键并保留注释、换主题清掉颜色覆盖、Default、写错值报错、文件被外部改过时拒绝保存、新键 `refresh_ms` 和 `corral` 的读取与校验。现有测试照样通过；`cargo clippy --all-targets -- -D warnings` 无警告。
- 键盘、鼠标的实际操作留给用户体验。

## 不要做

- 不做遥测开关、Diagnostics、Plugins、Updates。
- 不读写 Saddle 的配置文件。
- 不改 Saddle 仓库。

## 完成记录

2026-10-05，paddock/main。用户看 General、Colors 截图后同意：“可以，合并并重新安装”。

- **做了什么**：
  - `settings.rs`：设置项清单（General 8 项、Colors 主题加 61 个颜色按五组、Advanced 的 corral 命令）、草稿、换主题清掉颜色覆盖、Default（写默认值；颜色改为跟随主题即删键）、按类型校验、用 `toml_edit` 只写改过的键并保留注释和值的行尾注释、文件被外部改过时拒绝保存及 Keep/Discard。
  - `settings_view.rs`：窗口内设置面板（左侧页面、右侧设置项；开关、分段选择、色块、`custom` 与 `Restart required` 标记、单位、Default；底部提示与 Cancel/Save、冲突时 Keep my edits / Discard my edits）；保存写临时文件后改名。
  - `text_input.rs`：改自 GPUI `examples/input.rs`（Apache-2.0，保留来源说明）的单行输入框，加主题色、改动事件，复制粘贴走菜单动作。
  - 配置新增 `refresh_ms`、`corral`；`--corral` 优先于配置。保存后主题、颜色、侧栏宽度、宠物立即生效（侧栏、窗格换主题，宠物重建）。
  - 菜单 Settings…（⌘,）、⌘S 保存、Esc 取消（设置面板的按键上下文）；`app/README.md` 同步。新增依赖 `toml_edit`、`unicode-segmentation` 均已在锁文件里，未增加包。
- **验证了什么**：118 项测试通过（设置 7 项、配置 1 项）；clippy、fmt 通过。用临时配置和临时演示代码打开三页设置、只截本窗口；演示代码截图后删除、未提交；截图给用户看过，未入库，临时配置已删。
- **拿主意的地方**：设置做成窗口内面板而非单独窗口；配置路径显示为 `~/…`；数字项显示单位。
- **没做的事**：遥测开关、Diagnostics、Plugins、Updates；输入框里打字、切换和保存的实际操作留给用户体验。
