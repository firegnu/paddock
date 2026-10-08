# 任务：New Agent 改成主窗口里浮起的输入面板；预设改过后仍保持选中

2026-10-08，paddock/main 交给 paddock/dev-newagent（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：档拿不准（常规 0.79），交叉审查不要，影响面改行为；档按规则定常规）
类型：功能变更
依据：`docs/DESIGN.md` §13 最后一条（P5-43，取代 P5-37 的独立窗口）；样稿 `docs/设计稿/P5-43-NewAgent浮层/`。开 agent 仍走 `corral start`，命令、标签规则照 P5-21／P5-37。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」。
- `docs/DESIGN.md` §13 的 P5-37（New Agent 重新设计）和最后一条 P5-43；P5-4／P5-10 弹出界面的样式约定。
- 样稿：`docs/设计稿/P5-43-NewAgent浮层/` 的 `A-Overlay.dc.html`（平时）、`A-Picker.dc.html`（点开胶囊、预设 edited），README 说明取舍。
- 代码：`app/src/new_agent_view.rs`（现在的 New Agent 窗口，P5-37 做的）、`app/src/new_agent.rs`（拼命令、预设、`Form::matches`）、`app/src/windows.rs`（`open_new_agent`、`new_agent_busy`、设置保存后 `restyle`）、`app/src/window.rs` 里命令面板（`Palette`，主窗口内浮层、压暗、焦点、Esc 的现成做法）和三处 `open_new_agent` 调用、`app/src/window_ctl.rs`（ctl 的 busy 读 `new_agent_busy`）、`app/src/menu.rs`（⇧⌘N、New Agent 窗口里的 ⌘W／Esc／Return 绑定）、`app/src/popover.rs`、`app/src/text_input.rs`。
- P5-37 的任务文件和完成记录：`docs/任务/P5-37-NewAgent重新设计与真实图标.md`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-43-newagent-overlay`，分支 `p5-43-newagent-overlay`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-43-newagent-overlay`。

## 要做的
用户 10-08：“新建agent的面板，选择了controller/Developer/Codex bulder之类的，再选下面的强度。上面的选定的焦点就丢了。而且我还是觉得这个新建agent的不好看，总给人一种10年前对话框的感觉。”主控截图指出旧感的来源（独立系统窗口带标题栏、固定 640×600 下半截空着；输入区框套框；描边方块预设；选项前无含义的小色块；Show command／Cancel／Create 底栏；辅助文字多），出两种样稿（A 主窗口浮层、B 精简独立窗口），用户：“两种都画出来比较”，看过后：“选A，写任务文件吧。”

**1. 从独立窗口改成主窗口里的浮层**
- ⇧⌘N、菜单 New Agent…、侧栏底部菜单、窗格里新建 agent 的入口（`window.rs` 三处）都改为在主窗口里打开浮层，不再开系统窗口；原来带的打开位置（`Place`）照旧传进来。已经开着时再触发，只把焦点放回输入框（照现在“已开着就提到前面、保留已填内容”的意思）。
- 主窗口整体压暗，面板居中偏上（样稿 A：顶边约在窗口高度的 18%），宽约 680pt、随界面字号缩放；窗口太小时收窄、不超出窗口。高度随内容，不留空白。点压暗的地方关闭（有未发送的文字时也直接关，内容保留到下次打开，同现在关窗口后的行为一致即可）。
- 打开时焦点在大输入框；关闭后焦点回到打开前的窗格。浮层开着时，键盘不进终端；⌘W、⌘T 等主窗口快捷键不要误关窗格或开标签（⌘W 改成关浮层，或不响应，二选一，写进完成记录）。
- 设置保存后，开着的浮层跟着换主题、终端字体和预设（现在由 `windows.rs` 转给 New Agent 窗口，改成转给浮层）。
- ctl 的 busy（`new_agent_busy`）照旧：浮层正在创建时算 busy。

**2. 照样稿 A 的样子**（只从主题取色；界面字号 13 和 18 都放得下）
- 上面一行：预设做成轻量标签（种类图标＋名字，无方框，选中的有底色），末尾 “+” 存当前设置为预设、悬停出 × 删除（沿用 P5-37 的增删）。预设多到一行放不下时收进 “+N”。右边是名字小字（`前缀/名字`，等宽，带铅笔），点一下就地改（沿用 P5-37 的改名）。
- 中间：大输入框，多行、可留空，没有内框描边；占位 “What should it work on?”。
- 底边一行胶囊：项目（文件夹图标）；**合并的“种类图标 模型 · 强度”**——点开一个弹出框，上面选种类（Claude／Codex／pi／omp，pi、omp 注明用自己的默认），下面模型、强度做成分段按钮（样稿 A-Picker）；pi、omp 时胶囊只显示种类；打开位置（Split right／New tab／This pane）；角色做成 Controller 开关（关时虚线、开时实心）。右边 `</>` 展开命令（面板里往下展开：可手改的命令和完整 `corral start …`，沿用 P5-37 的同步规则），最右是圆形发送按钮（↑），⌘↩ 同样创建。
- 面板下面一行很淡的按键提示（⌘↩ start · esc close · 留空就开成空闲），可选。
- 去掉 Cancel 按钮和底栏；Esc 先收起弹出框或小输入框，没有打开的才关浮层（同现在）。
- 去掉的旧东西：独立窗口和它的标题栏、“name · click to rename”“Leave empty to start it idle”“Opens as …”这些辅助文字、选项前的小色块。

**3. 预设改过后仍保持选中**（用户说的“焦点丢了”）
- 记住最后点的是哪个预设。之后改了种类、模型、强度或角色中的任何一项，它仍然显示为选中，旁边加小圆点和 “edited”，并出现 “Update preset”：点了把当前设置存回这个预设（同名覆盖，沿用保存逻辑），存完 edited 消失。
- 改回和预设一样时，edited 自动消失。点另一个预设就换成那个。没点过预设、当前设置又恰好和某个预设一样时，照现在点亮那个。
- 手改命令后同步出来的值也算“改过”。

## 怎么算做完
- 用户原话：“新建agent的面板，选择了controller/Developer/Codex bulder之类的，再选下面的强度。上面的选定的焦点就丢了。而且我还是觉得这个新建agent的不好看，总给人一种10年前对话框的感觉。”“两种都画出来比较”“选A，写任务文件吧。”
- 第 3 条先写会失败的测试（点 Developer → 改强度 → 仍选中且 edited；改回 → edited 消失；Update preset 后存回且 edited 消失；点别的预设换过去），再实现。P5-37 已有的测试照旧通过。
- `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 都过。
- 截图（只截自己的测试窗口，临时 HOME／XDG、假 corral、`PADDOCK_NO_ACTIVATE=1`；需要临时开关让启动后自动打开浮层的，截完撤掉、不提交）：浮层平时的样子；点开“种类·模型·强度”胶囊；预设 edited 的样子；界面字号 18。截图放 scratchpad，不入库。
- 键盘、鼠标、输入法、点外面关闭、焦点回到窗格这些交互，留给用户在真窗口里试。

## 不要做
- 不改拼命令、标签、预设存储格式、项目列表、种类图标的规则（P5-21／P5-37 的行为保持）。
- 不加新依赖；不用 Python。
- 命令里不用 `rm`，不用 `sh -c` 包长命令。
- 不碰别的窗口（Settings、About）的形式。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
