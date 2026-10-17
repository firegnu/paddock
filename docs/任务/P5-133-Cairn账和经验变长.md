# 任务：Cairn 面板的 Ledger、Lessons 改成一行一条，分组可收起，加过滤框

2026-10-17，paddock/main 交给 paddock/dev-cairn-compact（Claude Code，常规：opus[1m] / high）。
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规、拿不准、拿不准；主控按规则定：只动 Cairn 面板的显示和一个本机偏好文件，不碰数据模型、公开约定和并发）
类型：功能变更
依据：用户 10-17：“另外Ledger和lessons的显示现在是完全平铺，随着item越来越多，会不会显得很长？你要设计一下。”主控出了样稿（画布「P5-133 Cairn 账和经验变长样稿」，源文件在 `docs/设计稿/P5-133-Cairn列表变长/`），A 一行一条 + 分组可收起 + 过滤，B 保留预览每组露 5 条；用户：“选 A，只展开 State，组内新的在上”。
执行：派出
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

派出的理由：面板列表的画法、过滤框和收起状态，界面实现按分工给 Claude Code。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部。
- 样稿：`docs/设计稿/P5-133-Cairn列表变长/README.md` 和 `A-Compact`、`A-Filter`、`A-Lessons` 三张；`Main` 是现在的样子。
- `docs/DESIGN.md` §13 P5-118 那条（含 P5-118a、P5-121、P5-121a 的补充）。
- 代码：`app/src/cairn_view.rs`（`entries`、`entry`、`ledger`、`lessons`、`switch`）、`app/src/cairn.rs`（`Group`、`Entry`、`ledger()` 里 P5-132 给 Lessons 排序那段）、`app/tests/cairn.rs`；文字框 `app/src/text_input.rs`，用法参照 `app/src/browser_view.rs` 的查找栏；⌘F 怎么分给 Browser：`app/src/menu.rs` 的 `finds`；本机状态文件的读写参照 `app/src/remote_start.rs` 的 `recent_path` 和 `RecentDirs`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-133-cairn-compact`，分支 `p5-133-cairn-compact`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-133-cairn-compact`（已从 `.target/main` 克隆）。
- 动 `app/src/` 下要动的文件、相关测试、`docs/DESIGN.md` 和本文件。不加依赖。

## 要做的（照样稿 A；颜色从主题取，样稿色值只是示意）
1. **一行一条**（Ledger、Lessons 两页）：每条一行，折叠箭头、`#编号`（等宽、右对齐，照旧）、标题（一行，过长省略号）、右边淡色日期（取 Added 的日子，如 `Oct 16`；不是今年的带年份）。不再画两行正文预览。点一条就地展开、同时只开一条、展开后的正文和 Added／Updated／By／Entry、没带上的那行，都照现在。下个会话没带上的标题照旧画淡。
2. **组内新的在上**：Ledger 每组都按编号从大到小（Lessons 已是，P5-132）。
3. **分组可收起**（Ledger 页）：组头整行可点，前面 `▾`／`▸`，后面条数小签照旧；收起时只剩组头。默认只展开 State，Open、Conventions、Other 收着。用户点过的收起状态**每个仓库各自记住**，paddock 重开后还在：存一个本机状态文件，和 `recent-remote-dirs.json` 放一处、互不干扰；读不到或坏了当没有，用默认。Lessons 页只有一组，不画组头，不可收（照现在）。
4. **过滤框**（两页各一个）：在 `Notes | Ledger | Lessons` 切换下面，占位字 `Filter <N> entries`（Lessons 页 `Filter <N> lessons`），右边 `⌘F`。
   - 按标题和正文过滤，不分大小写，按子串。有字时：框描边用强调色，右边写 `<命中> of <总数>` 和一个清除 `×`；组头条数写 `<命中> of <该组总数>`；标题里命中的字高亮；没有命中的组不显示；收着的组有命中时照样展开显示（不改记住的收起状态，清掉过滤后恢复）。一条都没命中时，列表处写 `No entries match “<字>”`（Lessons 页 `No lessons match …`）。
   - Esc 清掉过滤；点 `×` 也清掉。
   - ⌘F：键盘在 Cairn 面板里时进这一页的过滤框；在别处照旧是终端、Browser 的查找（照 `menu::finds` 给 Browser 的分法）。
   - Ledger、Lessons 的过滤字各自记着，换仓库清空（照现在换仓库复位开着的那条）。不存盘。
5. `docs/DESIGN.md` §13 P5-118 那条后面补一条 P5-133，写清以上（顺带把 P5-132 Lessons 新的在上写进去，如果还没写）。

## 怎么算做完
- 用户原话：“选 A，只展开 State，组内新的在上”。
- 验证只做这些：针对这件活写测试，覆盖过滤（标题、正文、不分大小写、命中数、组没命中时不显示）、组内顺序（编号大的在上）、收起状态（默认只开 State、按仓库分开、读写状态文件，读不到或坏了当默认）、日期短写法（今年和非今年）。`git diff --check`，`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑过一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改 Notes 页；不改切换胶囊的画法；不加写账的命令；不调 `cairn` 的新命令或参数（只用现在的 `ledger --json`），不读 cairn 的数据库和配置。
- 不改、不读用户真实的 `~/.config/paddock/` 下的配置和状态文件，测试一律用临时目录；不起 paddock 窗口截图（样子留给用户看）。
- 不改终端、Browser 的 ⌘F 行为（键盘不在 Cairn 面板时）。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西；不要按项目名或路径批量杀进程。
- 遇到要改公开约定（`cairn` 的命令或输出）才能做的，停下来报告。
- 不合并到 main，不推送。只在 `p5-133-cairn-compact` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的，工具或环境和预想不一样的；没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
