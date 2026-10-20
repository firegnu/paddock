# 任务：设置 › Sandbox 页显示套墙的 agent 查资料用的是哪家 MCP 服务器（内置还是本机加的）

2026-10-20，ranch/main 转来用户的需求，主控写任务书；用户看过后问“这个活难吗？”，主控答是小活，用户：“派给omp做吧”。paddock/main 交给 paddock/dev-search（omp，默认档，不套墙）。
依据：ranch `docs/需求/paddock-设置里管理查资料的服务器.md`（10-20 ranch/main 写）；字段以 ranch `docs/CONTRACT.md`「sandbox 与 `fence`」一节 R30 那段为准。ranch 不用改，用现装 fence（`versions/2f52652`）现有的输出就能做。
执行：派出
（理由：用户点名交给 omp。不套墙：套 fence `dev` 墙的 omp 起来十几秒就停（P5-150 时遇到，已交 ranch 查）。主控审，装上给用户看。）
路由：常规 / 交叉审查不要 / 影响面：改行为（路由：常规（置信 0.88）、不要、改行为）
类型：功能变更
提示：围绕已确认的使用目标完成变更，优先沿用现有机制。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 用户原话
- 10-20（对 ranch/main）：“另外你觉得settings是否可以加上search的配置？”
- 10-20（对 ranch/main）：“按你的建议，只做显示，测试按钮先不做”

## 要的结果（ranch 需求原文）
Sandbox 页上看得出套墙的 agent 查资料用的是哪一家，以及它是内置的还是本机加的，例如「查资料：exa（内置）」。

## 现在是怎样（主控 10-20 对着代码和本机 fence 输出核对的）
- `app/src/settings_sandbox.rs`（P5-120）：Settings › Sandbox 只看不改。上面 fence 版本；`Profiles` 一节列 `fence profiles` 的档案（来源、说明、读不了的原因）和 `This Mac's additions`（`~/.fence/local.toml` 在不在）；选一个档案、种类、文件夹后，`fence check` 的结果：`Built from` 一排层的小标签（`profile dev + agent claude + … `）、网络、读写的规则卡片等。
- `app/src/fence.rs`：`Profiles` 只解析 `profiles`、`kinds`、`local`，不认 `mcp`；`Layer` 是通用的（`source`、`name`、`origin`、`path`），所以 `check` 的 `layers` 里 `source` 为 `mcp` 的那一层现在只作为 `mcp exa` 一个小标签出现在 `Built from` 里，没说它是查资料用的。来源的说法 `fence::origin`：`builtin` → `built in`，`user` → `yours`，别的照原样。
- 本机 fence 现在的输出：`fence profiles` 有 `"mcp":[{"name":"exa","source":"builtin"}]`；`fence check --profile dev --kind claude` 的 `layers` 末尾有 `{"name":"exa","origin":"builtin","source":"mcp"}`。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-160-search`，分支 `p5-160-search`（已从 main 建好）。只动 `app/src/fence.rs`、`app/src/settings_sandbox.rs`，要的话 `docs/DESIGN.md` §13 P5-120 那段补一句。
- 编译：`CARGO_TARGET_DIR=$HOME/Developer/personal_projs/paddock-worktrees/.target/p5-160-search`（从 `.target/main` 克隆）。

## 要做的
1. **有哪些**：`Profiles` 一节里（`This Mac's additions` 旁边，同样的小号淡色写法），加一行列出 `fence profiles` 的 `mcp` 数组：每家 名字 加 来源（`builtin` 是内置，否则是本机加的；来源的说法沿用 `fence::origin`）。数组里读不了的项照档案的写法写原因（红色）。
2. **这个档案实际用哪家**：`fence check` 的结果里（`Built from` 那一排附近），一行写这个档案和种类查资料用的是哪家：`layers` 里 `source` 为 `mcp` 的项的 `name` 加 `origin`；没有这一项就写没接查资料的服务器。
3. 文字用英文（界面文字的规矩），意思照需求：例如 `Web search: exa (built in)`、`Web search: none`；具体措辞开发定，和这页现有的说法一致。
4. 老版本 fence 的输出里没有 `mcp` 字段时，这两行都不出现，不报错。

## 怎么算做完
- 用户原话：“按你的建议，只做显示，测试按钮先不做”；需求原文：“Sandbox 页上看得出套墙的 agent 查资料用的是哪一家，以及它是内置的还是本机加的”。
- 主控建议的验收（请用户确认）：
  1. 针对测试（合成的 fence JSON，不调真 fence）：`profiles` 里的 `mcp` 解析出名字和来源（内置、本机加的、读不了的、没有这个字段的）；`check` 的 `layers` 里找得到 `mcp` 那一层、找不到时说没接。
  2. 三项检查全过（`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`）。
  3. 用户装上后在 Settings › Sandbox 看一眼：`Profiles` 下列出 exa（内置），选 `dev` 和一个种类后写着查资料用 exa（内置）。
- 验证只做这些：第 1 条的针对测试先写、先跑红再实现，三项检查各跑一次（`CARGO_TARGET_DIR` 用上面写的目录）。第 3 条留给用户，你不起窗口、不截图。觉得不够，在回复里说，不要自己加。

## 不要做
- 不在界面里换服务商、添加删除、填 key，不做测试按钮（用户 10-20 说先不做）。要换服务商照 ranch `crates/fence/README.md` 手改 `~/.fence/`，界面上不用写这个说明，除非开发觉得一句话的提示有必要，先问主控。
- 只调 PATH 上的 `fence` 命令、读它的 JSON，不读写 `~/.fence` 里的文件（包括 `keys/`）。
- 不改 Sandbox 页现有的其他部分；不改 ranch、fence。
- 不加新的主题颜色键，不加依赖。
- 不用 Python；命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不起 paddock 窗口，不碰 `corral` 里现有的 agent，不对任何 agent `corral send`。
- 不合并到 main，不推送。只在 `p5-160-search` 上提交。

## 做完
三项检查跑完，在本文件末尾追加「## 完成记录」：做了什么、验证了什么（哪些测试先红后绿）、拿主意的地方、没做的事、踩过的坑（试了不行换了办法的、工具或环境的行为和预想不一样的；没有就写「没有」），在你的分支里提交。主控审过后装上给用户看，用户说可以再合并。回复里只写这几样，加上有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
