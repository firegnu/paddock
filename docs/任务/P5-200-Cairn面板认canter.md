# 任务：Cairn 面板认 canter

2026-11-11，paddock/main 写。
类型：功能变更
依据：canter/main 转来的需求（用户 11-11 看过、同意交给 paddock：「你来转吧」），全文抄在下面「用户转来的需求」。cairn 那一半已做完：cairn P1-27（cairn main `7d44c8e`），约定见 cairn `docs/DESIGN.md` §7.1。本机现装 cairn 0.7.1，还没有 canter；新版由 cairn 主控装好后告诉用户。paddock 只照 cairn 的公开格式接，cairn、canter 都不用改。
执行：主控

## 用户转来的需求（canter，原文）

> # 需求：Cairn 面板认 canter
>
> 来自 canter 仓库的主控，用户 11-11 让 canter 主控把 paddock 这件交给 paddock。
>
> ## 用户原话
>
> - 11-11（canter 主控说明接 cairn 的顺序：canter 加钩子注入 → cairn 加 canter → 用户装钩子 → paddock 的 Cairn 面板认 canter）：「都按你的建议来」
> - 11-11：「关于接入cairn的事情，我还想问一下，其实就是canter接入cairn之后就是和现在接入cairn的codex和claude code对齐了呗」
> - 11-11（cairn 主控转达）：paddock 那边的任务由 canter 主控派。
>
> ## 起因
>
> cairn 的 P1-27（cairn main `7d44c8e`）把 canter 接成第三家 agent。paddock 的 Cairn 标签现在只读 `agents.claude`、`agents.codex`（P5-55、P5-78、P5-79），所以：
>
> - 只装了 canter 钩子的项目，标签显示没装钩子，记录和账都不显示；
> - 标签头只有 Claude、Codex 两家装没装、最近什么时候触发；
> - canter 存的记录，「谁存的」写成小写的 `canter`。
>
> ## 要的结果
>
> 1. canter 和 Claude、Codex 一样出现在 Cairn 标签里：装没装、最近触发时间照另外两家的样子显示。
> 2. 三家里任一家装了钩子，项目就照「装了」处理：采用、记录、账照常显示。
> 3. canter 存的记录、加的账，「谁存的」「谁加的」显示 `Canter`（和 `Claude`、`Codex` 同样写法）。
> 4. 旧版 cairn 没有 `agents.canter` 时，canter 那项当不知道，不报错；Claude、Codex 的显示和现在一样。
> 5. 机器上没装 canter 时，什么都不变（cairn 报 canter 没装，就照没装显示）。
>
> ## cairn 那边已有的（cairn `docs/DESIGN.md` §7.1，只加不改）
>
> - `cairn status --json` 新增 `agents.canter.installed`（布尔；没装、没有 canter 配置目录、配置读不了都是 false，不报错）和 `agents.canter.last_seen`（四个键 SessionStart / UserPromptSubmit / Stop / SessionEnd，值是 RFC 3339 或 null），和另外两家同样的形状。`agents.claude`、`agents.codex` 不变。
> - `cairn list --json` 的 `agent` 多一个取值 `canter`，`created_by` 可能是 `canter:…`。
> - cairn 装到真实环境后库升到版本 6，旧版 cairn（0.7.x）打不开新库；版本号由 cairn 主控装好后告诉用户。
>
> ## 不要做
>
> - 不读 canter 的状态目录或配置，只照 cairn 的公开格式接（paddock 跟着 cairn 的约定走，不是 canter 的）。
> - 不改 cairn、canter 仓库；需要它们改的写成需求交用户。

## 用户原话
- 11-11（对 canter 主控，经转述）：「都按你的建议来」；「其实就是canter接入cairn之后就是和现在接入cairn的codex和claude code对齐了呗」。
- 11-11：同意由 canter 主控把需求转给 paddock：「你来转吧」。
- 11-11 看过任务文件和主控另提的三条：「都按你的建议来，开工吧」。

## 先读
- 仓库根的 `CLAUDE.md`。
- `docs/DESIGN.md` §3 cairn 一行（第 48 行附近）、§13 P5-55、P5-78、P5-79。
- cairn `docs/DESIGN.md` §7.1 的 `agents.canter.*`、`list --json` 的 `agent`、`created_by` 几行（只读）。
- 代码：`app/src/cairn.rs` 的 `Found`、`Agents`、`Agent`、`read`、`fired`、`by`；`app/src/cairn_view.rs` 的 `hooks`、`body` 的 `Body::NoHooks`、采用卡片的说明文字。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-200-cairn-canter`，分支 `p5-200-cairn-canter`（已从 main 建好）。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-200-cairn-canter`（从 `.target/main` 克隆）。
- 主要动 `app/src/cairn.rs`、`app/src/cairn_view.rs` 和 `docs/DESIGN.md`。

## 要做的
1. **读**：`Agents` 多读一个可选的 `canter`。没有这个字段（旧版 cairn）就是不知道，`status` 照样读得出来，不报错。
2. **装没装**：`Found` 给 canter 一项，分三种：装了、没装（cairn 报 `installed: false`）、不知道（旧版 cairn）。最近触发时间照 P5-78 的 `fired` 算，只在已采用、装了的时候画。
3. **任一家装了就照装了处理**：现在 `!claude && !codex` 才是 `NoHooks`，改成三家都没装才是。只装了 canter 的项目照常读 `show`、`list` 和账。
4. **谁存的、谁加的**：`by` 加 `canter` → `Canter`。记录列表、点开的 `By`、账的「谁加的」走的都是它，一处改完。
5. **标签头**：`Hooks` 后面在 Claude、Codex 之后画 `Canter`，装了画勾和时间，没装画淡色短横，写法照另外两家。不知道（旧版 cairn）时不画这一项，Claude、Codex 照旧。
6. **采用卡片**：canter 装了钩子时，说明写成 “Claude, Codex and Canter sessions”；没装照旧（下面第 3 条）。没装钩子时的提示和命令不改（下面第 2 条）。
7. **DESIGN**：§3 cairn 一行补上 `agents.canter`，§13 加 P5-200 一条。

## 怎么算做完（照抄需求的「要的结果」）
1. canter 和 Claude、Codex 一样出现在 Cairn 标签里：装没装、最近触发时间照另外两家的样子显示。
2. 三家里任一家装了钩子，项目就照「装了」处理：采用、记录、账照常显示。
3. canter 存的记录、加的账，「谁存的」「谁加的」显示 `Canter`（和 `Claude`、`Codex` 同样写法）。
4. 旧版 cairn 没有 `agents.canter` 时，canter 那项当不知道，不报错；Claude、Codex 的显示和现在一样。
5. 机器上没装 canter 时，什么都不变（cairn 报 canter 没装，就照没装显示）。

验证办法：用假的 `cairn` 脚本和合成 JSON 测上面五种情况（只装 canter、三家都没装、旧版没有 `agents.canter`、canter 报没装、`canter:…` 的记录和账）。真实效果等 cairn 新版装好、用户装上 canter 钩子后由用户看；本机现装的 0.7.1 还没有 canter。合并前跑 `cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。

## 主控另提的三条（用户 11-11：「都按你的建议来」，照每条的建议做）
1. **标签头里 canter 没装和不知道怎么画**：主控建议没装就照另外两家画淡色短横；旧版 cairn 不知道时不画这一项（短横会让人以为“查过、没装”）。需求第 5 条写的是“什么都不变”，又写“照没装显示”：照这条建议，没装 canter 的机器上标签头会多一个淡色的 `Canter —`。cairn 分不出“没装 canter”和“装了 canter、没装钩子”。另一种做法是没装就不画，但这样和 Claude、Codex 没装时画短横不一致。
2. **三家都没装钩子时的提示**：现在写 “Nothing is recorded for Claude Code or Codex until they are”，下面列 `cairn install --agent claude`、`--agent codex` 两条命令。主控建议不改：要加 canter 就得每台机器都多一条，和第 5 条“没装 canter 什么都不变”冲突。
3. **采用卡片的说明**：现在写 “cairn summarizes the Claude and Codex sessions…”。主控建议在 canter 装了钩子时写成 “Claude, Codex and Canter sessions”，没装时不变，免得只装了 canter 的人以为它不算。

## 不要做
- 不读 canter 的状态目录、配置文件，也不查 `canter` 命令在不在；只看 `cairn status --json`。
- 不改 cairn、canter 仓库。
- 不加安装、卸载钩子的按钮（P5-55 定的不做照旧）。
