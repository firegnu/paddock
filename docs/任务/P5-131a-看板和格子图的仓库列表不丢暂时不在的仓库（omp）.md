# 任务：看板 Kept 和活动格子图的仓库列表不丢暂时不在的仓库，看板存不进去要提示（omp 做的一份）

2026-10-17，paddock/main 交给 paddock/dev-kept-omp（omp，默认）。
路由：常规 / 交叉审查不要 / 影响面：改行为（主控定，没问路由：用户点名派 omp、agy 比较；只碰本机一个小 json 文件的读写，做错了是列表不对）
类型：Bug 修复
依据：用户 10-17：“我想接下来测试一下omp和agy，你找一件任务”；主控提了看板 Kept 列表的两个毛病（HANDOFF「建议改未排」里 P5-85 留下的），并建议活动格子图共用的读法一起修，用户：“可以，两边一起修，开工吧。”同一件活另一份派给另一家，各做各的，主控比较后合并一份。
执行：派出
提示：依据证据定位并修复导致问题的原因，保持无关行为不变。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 现在是怎样（主控 10-17 读代码记的）
- 看板（右侧栏 Kanban）标题上的仓库名点开是一个菜单，选过的仓库记在 `Kept` 一段，存在本机 `kanban-repos.json`（`app/src/kanban_repos.rs` 的 `Looking`；DESIGN §13 P5-85：“读的时候目录已经不存在的跳过……每次开菜单、加、移之前先重读这份文件”）。左侧栏的活动格子图有一份同样的仓库列表 `activity-repos.json`（`app/src/activity.rs` 的 `Poller`），两边共用 `activity::load_repos`、`activity::save_repos`。
- 毛病一：`load_repos` 读的时候把目录不存在的过滤掉了，`Looking::choose`／`remove` 和格子图的 `Poller` 拿过滤后的列表改完再存回去，于是外接盘没插、目录暂时改名这类“暂时不在”的仓库，在下次加、移（格子图是发现新仓库）时被从文件里永久删掉。
- 毛病二：看板存列表失败时错误被丢掉（`let _ = activity::save_repos(...)`），用户不知道没存上。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-131a-kept`，分支 `p5-131a-kept`（已从 main 建好）。
- 编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-131a-kept`（已从 `.target/main` 克隆）。cargo 命令在 `app/` 下跑。
- 只动 `app/src/kanban_repos.rs`、`app/src/activity.rs`、`app/src/kanban_view.rs`、相关测试、`docs/DESIGN.md` 和本文件。

## 先读
- `AGENTS.md`（规矩全文）。
- 本文件全部。
- `docs/DESIGN.md` §13 P5-85 那条（看板切仓库）和 P5-32 那条里格子图的仓库列表。
- 代码：`app/src/kanban_repos.rs`、`app/src/activity.rs`（`load_repos`、`save_repos`、`remember`、`Poller` 里读写列表的地方）、`app/src/kanban_view.rs` 里画菜单和 `isn't in a Git repository` 提示的地方；测试 `app/tests/kanban_repos.rs`、`app/tests/activity.rs`。

## 要做的
1. 毛病一，看板和格子图两边都修：暂时不在的仓库留在文件里，不因为加、移或发现新仓库被删掉；只有用户在菜单上点 × 移掉的才从看板的列表里删。显示照旧：看板菜单的 `Kept` 里、格子图统计时，不在的仓库照旧不列、不数，等它回来自动又出现。读文件、别的窗口改过不覆盖的做法照旧。
2. 毛病二，看板：存列表失败时在菜单里给一句提示（照 `isn't in a Git repository` 那一行的画法和位置，英文，写明没存上和原因），下一次存成功后提示消失。格子图存失败要不要提示、怎么提示，你判断，写理由；不要为它新做界面。
3. 两边的修法尽量共用一处，不复制。
4. `docs/DESIGN.md` §13 P5-85 那条后面补一句改了什么（P5-131）。

## 怎么算做完
- 用户原话：“可以，两边一起修，开工吧。”
- 先写测试复现，改之前跑一次确认因为这两个毛病失败，再修：看板加、移时暂时不在的仓库留在文件里且不在菜单里列出、回来后又列出；格子图发现新仓库时暂时不在的仓库留在文件里；看板存失败时有提示、存成功后提示消失。测试只用临时目录。
- 验证只做这些：上面的测试；`git diff --check`；`app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次。觉得不够，在回复里说，不要自己加。

## 不要做
- 不读、不改用户真实的 `kanban-repos.json`、`activity-repos.json` 和 paddock 状态目录；测试一律用临时目录。
- 不改菜单的别的行为和样子、格子图的样子，不改 DESIGN 里 P5-85 别的规则。
- 不起 paddock 窗口、不截图；不碰 `corral ls` 里的任何 agent。
- 不用 Python；命令里不用 `rm`；不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）：主控和别的 agent 的进程命令行里都带着项目名和工作目录。
- 不看、不碰另一份同样的活（另一个 worktree 和分支）。
- 不加依赖；不合并到 main，不推送。只在自己的分支上提交，提交说明用中文、首行以本任务编号加冒号开头。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么（写明改前测试怎么失败的）、拿主意的地方、没做的事、踩过的坑（没有就写「没有」）。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
