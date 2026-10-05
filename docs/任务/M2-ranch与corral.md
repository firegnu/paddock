# 任务 M2：新建 ranch 仓库，迁入 corral，做到 Saddle 和 paddock 都能用

2026-10-05 起草，paddock/main 自己做（兼管 ranch）。待用户看过再动手。
类型：迁移（外部行为不变）＋新建仓库＋切换安装
依据：DESIGN §2、§3“步骤”第 2 步、§7 第 9 条；`docs/背景与决策记录.md` §6i、§6j。用户 10-05 原话：“选方案3，新建仓库，paddock主控兼管，名字你来选一个合适的。paddock前期负责把corral做到saddle和paddock都能用的地步。”

## 已定的（用户 10-05）

- 运行时独立成库，Saddle 和 paddock 都只是前端；新建仓库，由 paddock 主控兼管。仓库名由作者定为 **ranch**，放在 `../ranch`。
- 冻结点取 Saddle 当前 main `a31dea2`，不另与 Saddle 主控约定。
- 切换只改 `~/.local/bin/corral` 的指向；已在运行的 agent（包括主控自己）不做 `corral upgrade`，继续由原来那份管理。
- Saddle 定位为保底版：只保证和运行时对得上。

## 现状（10-05 只读核对）

- Saddle `crates/corral-core` 最后一次改动是 `9648ff0`（10-03）；已安装的 `711ab18` 与 `a31dea2` 中这部分相同。约 7400 行（源码、测试、README）；依赖 `libc`、`serde`、`serde_json`、`uuid`、`sha2`、`base64`、`regex`、`shell-words`，测试另用 `tempfile`；不依赖 Saddle 其他代码，没有 ratatui、crossterm、Python。
- `~/.local/bin/corral` 是软链接，指向 `~/.local/share/saddle/versions/711ab18/bin/corral`。
- corral 技能（`~/.claude/skills/corral`、`~/.agents/skills/corral`）由 `corral skills` 安装，内容编在程序里。
- paddock 按 PATH 调用 `corral`，不用改。Saddle 默认用和自己打包在一起的 corral，不找 PATH（`src/agent_program.rs`）；插件也是（`bundled()`）；Updates 页按配置里的 corral 比较和升级。

## 要做的

### 一、ranch 仓库（paddock 主控做）

1. 在 `../ranch` 新建 git 仓库，Cargo workspace，corral 放 `crates/corral/`（包名 `corral-core`、程序名 `corral` 不变）。以后遥测、插件协议、dispatch 也放 `crates/` 下。
2. 规矩文件：`AGENTS.md`（`CLAUDE.md` 链到它，同 paddock）、`docs/DESIGN.md`、`HANDOFF.md`、`docs/任务/`。规矩照 paddock：Rust stable、不用 Python、不依赖 ratatui、依赖由 ranch 自己管；编译目录 `$HOME/Developer/personal_projs/ranch-worktrees/.target/<子目录>`；改动要写明对两个前端的影响（DESIGN §3“Saddle 保底”）；不改 Saddle 和 paddock 以外的仓库。
3. 迁入 corral：
   - 照搬 `src/`、`resources/`、`tests/`、`README.md`；每个 `.rs` 文件开头注明来源（Saddle `a31dea2` 的原文件）。`resources/` 里的文件原样保留、不加注释（会编进程序或装到用户目录），来源写在 README。
   - 不迁 `tests/product.rs`（唯一的测试需要 Saddle 界面程序，测的是 Saddle 与 corral 的组合）。`tests/collectors.mjs`（Node，手动跑）照原样迁入。
   - 依赖版本以 Saddle 的 `Cargo.lock` 为起点，只删不升。
   - 只改说明文字（description、README 里“随 Saddle 发布”）；命令、JSON 输出、`~/.corral` 格式、退出码、技能内容都不动。
   - Saddle 的 `docs/Corral核心Rust集成设计.md`、`docs/Corral通用升级设计.md` 迁到 ranch 的 `docs/`。
4. 打包工具（Rust）：生成不可变版本目录 `~/.local/share/ranch/versions/<ranch 提交>/`，内含 `bin/corral`、`share/corral/`（资源和两份设计文档）、`BUILD.txt`（提交、目标平台、工作区是否干净、校验和）；目录已存在就拒绝。只生成目录，不切链接、不装技能。
5. 检查：`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。

### 二、切换（用户在场）

1. 记下 `~/.local/bin/corral` 原指向；生成版本目录；把链接改指向新目录的 `bin/corral`。出问题就改回去。
2. 技能：比较新程序要装的技能与现有目录，相同不动，不同先说明再装。
3. paddock 侧实测：自己开 `paddock/test-m2`（`--label role=test`，跑 `/bin/cat`），start / send / wait / reply / stop 各一次；paddock.app 侧栏照常列出 agent。

### 三、Saddle 侧（写需求交用户转 Saddle 主控，paddock 主控不改 Saddle 代码）

1. Saddle 开停接入 agent、插件都改用 ranch 装好的 corral（PATH 上的 `corral`），不再用自己打包的那份（`agent_program.rs` 的 `bundled()` 与默认配置）。
2. 打包不再含 `bin/corral`、`share/corral/`；部署不再切换 `~/.local/bin/corral`、不再装 corral 技能。
3. Updates 页按 ranch 装好的 corral 比较和升级，不再以 Saddle 打包的为准。
4. AGENTS.md 里“corral 由 paddock 维护”改为由 ranch（`../ranch`，paddock 主控兼管）维护；Saddle 内部的 corral 不能修改这条保留。Saddle 仓库里 `crates/corral-core` 留不留（例如留给 Saddle 自己的测试）由 Saddle 定，但不再打包发布。
5. Saddle 改完后，paddock 主控用测试 agent（`paddock/test-*`）在 Saddle 里核对：Saddle 能看到、接入、停止 ranch 的 corral 开的 agent；Saddle 开的 agent 由 ranch 的 corral 管（`corral status` 的 exe 在 ranch 版本目录）。

## 怎么算做完

- “paddock前期负责把corral做到saddle和paddock都能用的地步”（用户 10-05）。
- 命令、JSON 输出和 `~/.corral` 登记格式不变（DESIGN §3 第 2 步）。
- 切换 `~/.local/bin/corral` 时用户在场（DESIGN §7 第 9 条）。

## 次序与远程（用户 10-05）

- 先把 corral 从 Saddle 拉出去，直到 Saddle 能和新的 corral 一起工作，再转回 paddock 开发。第三节的需求现在就交 Saddle 主控，与第一、二节并行。
- ranch 建 GitHub 公开仓库（`github.com/firegnu/ranch`），不加许可证；推送前用 gitleaks、trufflehog 查隐私。

## 注意

- `paddock/main` 还在用 Saddle 版本目录 `aab70c8` 里的 corral，Saddle 主控的会话也在用 Saddle 打包的 corral；这些会话结束前，那些版本目录不能删。
- Saddle 改完之前，新旧两份 corral 同时操作同一批 agent；本次只搬代码不改行为，两份完全相同。

## 不要做

- 不改 corral 的行为、命令、输出和格式，不顺手重构、改名。
- 不把 corral 打进 paddock.app（单独分发时再定，DESIGN §7）。
- 不改 Saddle 仓库；不删 Saddle 的版本目录；不碰旧的 Python corral 仓库（`../corral`）。
- 不对用户的 agent 做任何写操作。

## 进度记录

2026-10-05，paddock/main。第一、二节完成；第三节（Saddle 侧）等 Saddle 主控。

- **第一节**：ranch 建在 `../ranch`，推送到 `github.com/firegnu/ranch`（public，不加许可证；推送前 gitleaks、trufflehog 查全部历史和文件，无发现）。corral 迁入 `crates/corral/`：先原样复制一次提交（`1309725`），再加来源注释（`df46247`）；核对过源码相对 Saddle 只多每个 Rust 文件开头一行注释。锁文件以 Saddle 的为起点只删不升，剩 53 个包。打包工具 `tools/package`（`ranch-package`，只用 std）。42 项测试、clippy、fmt 通过；`collectors.mjs` 用 Node 24 手动跑通过。
  - 第一次跑全部测试时 `protocol` 有一项在并行时卡住 7 分钟（三个测试 pen 空等）；按 PID 停掉后逐项跑、整体再跑 6 次都在几秒内通过，未能复现。是 Saddle 原有代码，留意。
  - 版本目录 `~/.local/share/ranch/versions/df46247/` 从干净的 main 生成；随程序文件与 Saddle `711ab18` 装的逐字节相同；已装的 corral 技能（`~/.claude/skills/corral`、`~/.agents/skills/corral`）与之相同，未重装。
- **第二节**（用户在场，10-05）：`~/.local/bin/corral` 从 `~/.local/share/saddle/versions/711ab18/bin/corral` 改指向 ranch `df46247`。`corral ls` 照常；`paddock/test-m2` 用 `/bin/cat` 测 start/status/send/wait/stop，用 claude 测 `--prompt` 首句、`send`、`wait`、`reply`（回复正确、状态到 idle），exe 都在 ranch 版本目录；用完 stop。退回：链接改回 Saddle `711ab18`。
- **还没做**：第三节，Saddle 改用 ranch 的 corral 后核对；paddock.app 侧栏未截图核对（用户可直接看）。
