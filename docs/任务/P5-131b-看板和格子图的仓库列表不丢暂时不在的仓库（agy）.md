# 任务：看板 Kept 和活动格子图的仓库列表不丢暂时不在的仓库，看板存不进去要提示（agy 做的一份）

2026-10-17，paddock/main 交给 paddock/dev-kept-agy（agy，默认）。
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
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-131b-kept`，分支 `p5-131b-kept`（已从 main 建好）。
- 编译：命令前加 `CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-131b-kept`（已从 `.target/main` 克隆）。cargo 命令在 `app/` 下跑。
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

## 完成记录
- **做了什么**：
  1. `app/src/activity.rs`：`load_repos`（及新增 `read_repos`）读列表文件时不再过滤掉当时不在磁盘上的目录，完整保留文件里的路径；新增共用的 `present_repos` 过滤当前在磁盘上的目录，供 `Poller` 统计和 `kanban_repos` 显示共用；`Poller` 在发现新仓库并写回 `activity-repos.json` 时保留暂时不在磁盘上的仓库，存失败时记 `unsaved` 以便下一轮重试。
  2. `app/src/kanban_repos.rs`：`Looking` 区分文件里的完整列表 `all` 和当前在磁盘上的 `kept`（通过 `activity::present_repos` 同步），`choose`、`remove` 改完完整列表 `all` 后再写回 `kanban-repos.json`，不再把暂时不在的仓库从文件里删掉；`menu` 和 `kept()` 仍只列出当前在磁盘上的仓库，目录回来后自动重新出现。`Looking::save` 记录写文件错误为 `Couldn't save the list: <原因>`（`Looking::save_error`），下一次写文件成功后清空；上一次保存失败未落盘时，`refresh` 不拿磁盘旧内容冲掉内存里的改动，`choose`／`remove` 仍会重试保存。
  3. `app/src/kanban_view.rs`：`repo_menu` 在菜单底部把 `self.looking.save_error()` 与 `menu.note` 一起按 `<目录名> isn't in a Git repository` 同一位置和样式画出。
  4. `docs/DESIGN.md`：在 §13 P5-85「记住什么」一段后补上 P5-131 的改动说明。
  5. `app/tests/kanban_repos.rs`、`app/tests/activity.rs`：补充看板加、移时暂时不在的仓库留在文件里且不在菜单列出、目录回来后又列出，看板存失败提示与存成功后消失，以及格子图 `Poller` 发现新仓库时暂时不在的仓库留在文件里、目录回来后恢复统计的测试（均只用临时目录）。
- **验证了什么**：
  1. 改前先跑复现测试确认因两个毛病失败：
     - `cargo test --test activity`：`the_list_keeps_paths_only_and_skips_those_gone` 在 `tests/activity.rs:204` 失败，`b` 目录暂时删掉后 `load_repos` 只返回 `[a]` 而丢了 `b`。
     - `cargo test --test kanban_repos`：`kept_repositories_are_saved_as_paths_and_read_back` 在 `tests/kanban_repos.rs:153` 失败，`b` 暂时不在时选 `d` 后文件里只剩 `[c, d]`、`b` 被永久删掉；`a_failed_save_leaves_a_note_until_a_save_succeeds` 在 `tests/kanban_repos.rs:200` 失败，`save_repos` 失败后 `looking.save_error()` 为 `None`。
  2. 改后跑通过：
     - `cargo test --test kanban_repos --test activity`（两套共 10 个测试全过）。
     - `git diff --check`（无输出）。
     - `app/` 下 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 各跑一次，全部通过。
- **拿主意的地方**：
  1. 看板存失败提示文字定为 `Couldn't save the list: {why}`，和 `kanban.rs` 的 `Couldn't write the file: {why}` 及菜单原有的 `Remove from this list` 用词一致；不改菜单原有的开关行为（点仓库照旧关菜单切看板、点 × 照旧留着菜单），提示留在 `Looking` 上直到下一次保存成功。
  2. 活动格子图存 `activity-repos.json` 失败不在界面上提示，只由 `Poller` 记下 `unsaved` 在后续轮询时自动重试。理由：格子图发现仓库是后台轮询顺带记下的，不是用户点出来的动作；面板本身是固定高度的小卡片、没有菜单或提示位（任务也要求不为它新做界面）；当次运行内存里的 `repos` 已经加上，统计不受影响，重启后只要 agent 还在该仓库也会再记上。
- **没做的事**：没有。
- **踩过的坑**：没有。

## 主控审查（和 P5-131a 比较）

- 用时约 27 分钟（02:05→02:32）。主控在分支上重跑 `cargo test --all-targets` 726 过、clippy、fmt 过，和完成记录一致。改前测试真红，三条都写清了失败处，毛病二也做到了先红。
- 做得好的：格子图存失败用 `unsaved` 下一轮重试；存失败后再选、再移会重试保存；两句提示能同时显示；格子图测试直接开真 `Poller` 跑，覆盖暂时不在、回来后又数。
- 问题：1. `refresh` 在上次存失败时不再重读文件，这期间别的窗口改的会被下一次成功保存盖掉，违反任务里“读文件、别的窗口改过不覆盖的做法照旧”（只在存失败之后才会遇到，记建议改）；2. 同时留 `all` 和 `kept` 两份状态，`menu` 在有文件时又从 `all` 现算一遍，`kept()` 返回的缓存和菜单可能不一致，每画一次菜单都对每个仓库 stat 一次；3. 读不成文件但文件存在时保留内存里的旧列表，改了原来的行为，没说明；4. 汇报里夹满 `file://` 链接。
- 结论：两份都把两个毛病修好了，这份不合并，选 P5-131a；按看板约定记“不做：比较落选”。
