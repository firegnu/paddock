# 任务：侧栏卡片的 agent 名字后面加种类小图标

2026-10-06，paddock/main 交给 paddock/dev-kind-icons（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：看得见（路由：常规（0.96）；交叉审查不要；影响面看得见（0.8））
类型：功能变更
依据：本轮只在侧栏卡片名字后面加种类图标；其他地方（标签、窄条、palette）不加。
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `AGENTS.md`「规矩」一节（编译目录、桌面窗口测试、`PADDOCK_NO_ACTIVATE`、不模拟按键、仓库公开）。
- `docs/DESIGN.md` §13 的原则，以及「P5-12」两条（含用户 10-06 的决定）。
- `docs/调研/P5-12-agent种类logo.md`（各家官方图形的来源和条件）。
- `app/src/card.rs`、`app/src/sidebar.rs`（卡片第一行：名字、未读点、本窗口标记、`claude · high`）；`app/src/footer_icon.rs`（现有图标怎么画）；主题里 `claude`、`codex`、`pi`、`omp` 四个种类颜色（`preset.rs`）。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-12-kind-icons`，分支 `p5-12-kind-icons`（已从 main 建好）。
- 编译目录：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-12-kind-icons`。
- 可以改：`card.rs`、`sidebar.rs`，新建一个图标模块（如 `app/src/kind_icon.rs`，在 `lib.rs` 注册）和 `app/assets/kinds/` 目录。

## 要做的
用户原话：“每一个agents名字后面的agent的类型（claude，codex，pi，omp等等）最好能加上官方的logo，紧跟在agent的名字后面。”“最好有一个大小合适的官方logo”。用户 10-06 定：pi、omp 用官方的；claude 和 codex 不放官方 logo（公开仓库，Anthropic、OpenAI 的条款不许），由我们画接近的。主控定：claude、codex 的图形要**原创**，只借颜色和意思，形状不照抄官方标志，不能让人误以为是官方的。

1. **四个图形**，每个一个 SVG 文件放进 `app/assets/kinds/`：
   - `pi.svg`：pi 官网 press kit 的 badge，`https://pi.dev/favicon.svg`（单色方标，官方说明用于 favicon 和小徽章）。下载原文件，图形不改；它靠 CSS 切换深浅色，需要的话只去掉这段 CSS、改成单一填充，写进出处说明。
   - `omp.svg`：`https://github.com/can1357/oh-my-pi` 仓库 `assets/icon.svg`（MIT）。下载原文件；同时把该仓库 LICENSE 的版权声明和许可全文放进 `app/assets/kinds/LICENSE-omp`。
   - `claude.svg`：原创。一个简单的圆角星形“火花”（例如 6 或 8 条圆头的短射线），不画成 Anthropic 的 Claude Spark 的样子（不要它那种长短不一、手绘感的放射线），也不用像素小怪兽。
   - `codex.svg`：原创。圆角小方块里一个终端提示符 `>_`，线条圆头。不画 OpenAI 的花形或六边形结。
   - 原创的两个用简单路径，在 14pt 左右看得清；四个视觉分量差不多（大小、线粗、留白）。
2. **出处说明** `app/assets/kinds/README.md`（英文）：每个文件的来源 URL 和下载日期、许可（pi 写“from the pi press kit, intended for compact badges”；omp 写 MIT 和版权人）；claude、codex 写 “original drawings for paddock, not the official marks; Claude and Codex are trademarks of Anthropic and OpenAI”；最后一句写明分发前要复核这些图标（用户 10-06：分发时会处理）。
3. **显示**：卡片第一行名字（和未读点、本窗口标记）后面紧跟种类图标，再后面照旧 `claude · high` 文字。图标高度和名字的字差不多（界面字号 13 时约 13pt），随 `ui` 缩放，和文字竖直居中；按主题里该种类的颜色着色（单色），选中和未选中一样。种类不认识或没有种类时不显示、不占位置。名字放不下时图标和种类文字一样让位（沿用 P5-8 的规则：先去掉强度、再去掉工具，图标保留到最后，和种类文字一起）。
4. 图标编进程序（例如 `include_bytes!`），不在运行时读文件、不联网。
5. 只从主题取色，不新增主题颜色键；三套预置主题、界面字号 13 和 18 下都要好看、不折行。

## 怎么算做完
- 上面五条达到；用户看过 claude、codex 两个原创图形的截图后再合并（主控转给用户）。
- 验证只做这些：`git diff --check`；为“种类到图标的对应、不认识的种类不显示”写一条测试；在 `app/` 下跑一次 `cargo test --all-targets` 和 `cargo clippy --all-targets -- -D warnings`；用 `PADDOCK_NO_ACTIVATE=1`、临时 HOME 和假 corral（四种种类各一个 agent）起自己的窗口截侧栏（13 和 18 各一张），另把四个 SVG 放大画一张对照图（例如用 GPUI 把四个图标各画成 64pt 的临时窗口截图）；截图放 scratchpad，不入库，路径写进完成记录。觉得不够，在回复里说，不要自己加。

## 不要做
- 不下载、不放进仓库 Anthropic 或 OpenAI 的任何官方 logo、图标或它们的描摹版本（包括 press kit、favicon、Simple Icons 里的）；不照着官方图形描。
- 不改标签、窄条、palette、卡片第二行和详情；不改其他文件。
- 不新增主题颜色键，不加依赖，不改 `Cargo.lock`。
- 不要用 `osascript`、System Events 等任何方式模拟按键或鼠标。截图只截自己开的窗口，启动时加 `PADDOCK_NO_ACTIVATE=1`；用临时 HOME 和假 corral。
- 写给 Bash 的命令里不要用 `rm`，也不要把一长串命令包进 `sh -c '…'`。临时文件留在 scratchpad 里。
- `corral ls` 里的 agent 都是用户的，不对它们 stop/send/keys，不 attach 上去打字。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。停自己起的进程用记下的 PID。
- 不重新打包、不安装 paddock.app。
- 遇到做不到的（例如 GPUI 画不了这些 SVG、下载不到原文件），停下来报告，等决定。
- 不合并到 main，不推送。只在 `p5-12-kind-icons` 分支上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事，各几句话。回复里只写这几样，加上截图路径和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
