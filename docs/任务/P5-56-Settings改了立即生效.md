# 任务：Settings 改了立即生效，不用再点 Save

2026-10-09，paddock/main 写，主控自己做。
类型：改行为
依据：
- 用户 10-08：“settings现在修改之后还要save。我需要修改就起效。”
- 用户 10-09：“开始P5-56，这个里面你注意下坑，有些如色彩这种不一定修改的有效，要注意fallback（不但是颜色可能其他也有）。”
- 用户 10-09 四项都选主控建议：输入框回车或离开时生效；换主题照旧清掉自定义颜色、保存条给 Undo；数字加上限；没装的字体标出来。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-56-settings-live`，分支 `p5-56-settings-live`（已从 main 建好）。

## 现状
- `settings.rs` 的 `Draft` 攒着改动，Save（⌘S、保存条）时只写改过的键（`toml_edit`，保留注释），整份再用 `Config::parse`、`Theme::from_config` 校验一遍才写；写完发 `SettingsEvent::Saved`，主窗口 `apply` 立即用上（颜色、侧栏宽、宠物、界面字体、终端字体），Refresh interval 和 corral command 要重启。
- 保存条：几处改动、Revert、Save；Saved／Not saved／重启提示；磁盘冲突时 Keep my edits／Discard my edits。关 Settings 窗口和 ⌘Q 有未保存改动时问 Save／Don't Save／Cancel。
- 输入框（数字、颜色、corral command）每敲一个字就进草稿；不合法时框变红、标签下写原因。

## 坑：改了不一定生效，会退回别的值
1. **颜色**：敲到一半的值不合法（`#ff0` 之前的 `#ff`）；还有敲到一半恰好合法的（想敲 `redish`，敲到 `red` 时已合法）。逐字生效会让界面闪成中间值，最后不合法时“退回上一个合法值”退回的也是中间值。
2. **数字**：敲 `14` 的途中先是 `1`。终端字号 1 会让所有终端重排、给 agent 发尺寸变化，Claude Code 等界面跟着重画；界面字号 1 连 Settings 窗口自己都缩成一团。`Config::parse` 只要求大于 0，`0.01` 也能写进去；步进器有下限（字号 6、行高 0.5、刷新 250），直接输入没有。
3. **侧栏宽度**：比当前界面字号下的最小宽度窄时，主窗口按最小宽度画（`sidebar::fit_width`），但文件和 Settings 里显示的还是敲的值，显示的和实际的不一致。
4. **字体**：Fallback fonts 是自由输入，没装的字体静默跳过，加了看不出没生效；终端字体和界面字体是从已装的列表里选，配置文件里手写了没装的名字时同样静默退回。
5. **换主题会清掉自定义颜色**（P5-44 沿用 Saddle 的规矩）：以前有 Revert 可以反悔，立即生效后点一下主题卡片，自定义颜色就从文件里没了。
6. **要重启的两项**：corral command 敲到一半（`cor`）就写进文件，下次启动就用这个。
7. **配置文件本身有问题**（打不开、不是合法 TOML、有不认识的键）：现在 Settings 显示默认值、Save 拒绝写。立即生效后绝不能拿默认值盖掉用户的文件。
8. **文件被别处改了**：现在报冲突、让选 Keep／Discard；立即生效后没有“草稿”要保护了。

## 要做的
1. 去掉 Save／Revert、⌘S 和“Save Settings”菜单项、关窗口和 ⌘Q 时的 Save／Don't Save／Cancel。
2. 每次改动立即写文件并让主窗口用上，写法照旧：只写这一个键、保留注释、整份校验通过才写。
3. 点选类（开关、宠物、主题卡片、字体列表、步进器 −／+、Reset、Fallback 标签的增删）点了就生效。
4. 输入框类（数字、颜色、corral command）：敲的时候只在 Settings 里预览（颜色色块、Terminal 预览照旧跟着变），按回车、离开这个框、关窗口时才生效；不合法的不写，框变红、写原因；离开时仍不合法，框里退回正在用的值，保存条说一句“<名字> 不合法，仍用 <值>”。
5. 数字直接输入也守步进器的下限，低于下限算不合法（坑 2）。侧栏宽度生效时按最小宽度取整写入，框里显示实际用的值（坑 3）。
6. 写之前先读磁盘上的文件，把这一处改动套在它上面写（别处改的内容保留），不再有冲突提示；文件坏了（打不开、不合法）就不写，保存条一直显示问题，直到文件好了（坑 7、8）。Settings 窗口回到前台时重读一次文件，显示别处改过的值。
7. 要重启的两项照旧标 “after restart”，生效时提示 “Restart paddock for: …”。
8. 保存条只剩提示：生效的提示（要重启的、换主题清了颜色的）和问题（不合法、文件坏了、写不进去）。

## 用户定的（10-09）
9. 换主题照旧清掉自定义颜色（坑 5），保存条写 “Switched to <主题>; <N> custom colors cleared” 加 Undo：点了恢复原主题和原来的自定义颜色；下一次别的改动后 Undo 消失。
10. 数字加上限：界面字号 ≤ 24、终端字号 ≤ 72、行高 ≤ 3、侧栏宽度 ≤ 800；超了和低于下限一样算不合法。步进器 + 也停在上限。
11. 没装的字体标出来：Fallback 标签、终端字体旁写 “not installed”（变暗），照样写进文件，系统照旧跳过（坑 4）。界面字体从已装列表选，配置文件里手写了没装的同样标。

## 不做
- New Agent 的预设（`agent_presets`）不在 Settings 里，不动。
- 命令行 `--font` 等覆盖和配置文件的关系不变。
- Refresh interval、corral command 不改成不重启就生效。

## 怎么算做完
- 用户原话：“我需要修改就起效。”“注意下坑……要注意fallback”。
- 上面的坑各有测试：中间值、低于下限和高于上限、侧栏宽度取整、磁盘上被改过、文件坏了不覆盖、换主题清颜色和 Undo、没装的字体。
- `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。
- 真窗口里的手感（回车、离开输入框、点主题）留给用户试（主控不模拟按键）。

## 完成记录

- 做了什么：
  - `settings.rs`：`range()` 给数字定上下限（界面字号 6～24、终端字号 6～72、行高 0.5～3、侧栏宽度 10～800、刷新至少 250），`value()` 超出就报 “<名字> must be from A to B”／“at least A”，`problem()` 和写入用同一套检查。`Draft::commit(disk)` 把草稿套在磁盘上此刻的文件上生成要写的内容，之后以它为底、草稿清空；文件不是合法配置时报错、不写、草稿不动。`drop_invalid()` 丢掉写不进去的值并给出名字。`theme_setting()`／`restore_theme()` 记下和恢复主题加自定义颜色（Undo 用）。`discard()` 改为只清草稿；删掉 `unsaved()`（保存条的“几处未保存”没了）。`save()` 和 `Conflict` 保留，侧栏拖动写宽度还用。
  - `settings_view.rs`：点选类经 `edit()` 立即写；输入框打字只进草稿（预览），回车（`menu::OpenSelected`）或失焦（`cx.on_blur`）时 `commit()`，Escape（`menu::Cancel`）退回正在用的值。`commit()` 先 `drop_invalid()`（保存条红字 “Not valid, so not changed: …”），再读磁盘、写入、发 `SettingsEvent::Saved` 给主窗口；文件坏了就丢掉改动、保存条说明，绝不覆盖。写完侧栏宽度比当前界面字号下的最小宽度窄时，再按最小宽度写一次并提示。窗口回到前台时 `reload()` 重读文件。换主题清掉了自定义颜色（点当前主题也算）时保存条写 “Switched to <主题>; N custom colors cleared.” 加 Undo，Undo 在下一次写入后消失、提示不自动消失。步进器停在上限。Fallback 标签、终端或界面字体没装时变暗并写 “not installed”（按名字不分大小写比，字体列表还没读完或以点开头的隐藏字体不标）。保存条改为只显示提示（`message_bar`），删掉 Revert／Save、磁盘冲突的 Keep／Discard、关窗口的 Save／Don't Save／Cancel。
  - `windows.rs`：Settings 关窗口、退出 paddock 前调 `finish()` 把还在输入的值生效，不再问。`menu.rs`：删掉 `SaveSettings` 和 ⌘S。
  - DESIGN §13 加 P5-56 条。
- 验证了什么：先写 `settings.rs` 的 5 个新测试（上下限、套在磁盘新内容上写、坏文件不覆盖、丢掉不合法的值、换主题后恢复），放空函数确认它们因行为不对失败，再实现到通过。界面层的 3 个新测试（步进器停在上限、保存条两句话、字体是否已装）和实现一起写的，没有先单独看红。旧测试里 `font_size = 0` 等几条的报错改由范围检查给出，断言文字跟着换成新的文字（仍要求报错、不写）；删掉随功能一起去掉的 `unsaved` 和保存条计数两个测试。`app/` 下 `cargo test --all-targets`（462 项全过）、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`、`git diff --check` 都过。
- 拿主意的地方：
  - 生效时把所有还在草稿里的值一起写（同一时刻最多只有正在输入的那一个），点别处的按钮等于先离开输入框。
  - 写不进去（文件坏了、磁盘写失败）时退回文件里的值，不留一个“看着改了其实没生效”的值在框里。
  - 侧栏宽度上限按用户定的 800；拖动侧栏的上限仍是 560，两者不同，没改拖动。
  - 刷新间隔直接输入低于 250 现在算不合法（以前只有步进器拦），用户定的“下限照步进器”。
- 没做的事：没起真窗口截图（10-09 事故后主控暂不在 agent 会话里起测试窗口，见 P5-52）。回车、离开输入框、Escape、点主题和 Undo、“not installed” 的样子和手感都留给用户试。
