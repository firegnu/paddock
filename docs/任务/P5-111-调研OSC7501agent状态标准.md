# 任务：调研 Ghostty 作者提出的 OSC 7501（agent 状态的标准）

2026-10-15 用户提出，记成草稿；10-20 看板 Loop 送来开工，主控问范围，用户选“照这 5 条做”。paddock/main 交给 paddock/dev-osc（Claude Code，opus[1m] / high，套 fence `dev` 墙）。
依据：用户 10-15：“加一个draft task： 调研ghostty作者提出的OSC 7501， 这是agent状态的标准。”本轮只调研、写结论，不改代码，不改 ranch、corral。
执行：派出
（理由：要读外部资料、比较几家 agent 和终端，量不小；第 3 条要看 paddock 界面上能拿它做什么，带产品取舍，按分工表第 6 条给 Claude Code high。主控留着接 Loop 和收尾。）
路由：轻 / 交叉审查不要 / 影响面：看得见（路由：轻（置信 0.61）、拿不准、看得见；调研不改代码，一律看得见、不交叉审查）
类型：调研
提示：区分已查证事实、推测和未知，给出直接依据；结论限定在本轮调查范围。
你是被委派的 agent：照本文件做，不要再开别的 agent。

## 先读
- `docs/DESIGN.md` §3（和 Saddle、corral 的关系）；§13 里讲终端窗格标题、侧栏 agent 状态、收件箱的几条（搜 `OSC`、`title`、`Inbox`）。
- `app/src/corral.rs` 开头：paddock 现在从 corral 拿哪些状态字段。
- `app/src/` 里终端解析 OSC 的地方（搜 `osc`、`OSC 9`，P5-148b 加过 OSC 9;4）：看 paddock 内置终端现在认哪些序列、在哪一层接。

## 现状（主控 10-15 凭已知写的）
- paddock 现在拿 agent 状态只走 corral 的公开命令（`corral ls`／`status` 的 `state`、`last_tool` 等，DESIGN §3），不解析终端里的转义序列表示 agent 状态。
- 本仓库里没有提到 OSC 7501 的地方；主控没查过这份提案，下面的问题都等调研回答。

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-111-osc7501`，分支 `p5-111-osc7501`（已从 main 建好）。只新建 `docs/调研/P5-111-OSC7501调研.md`，并在本文件末尾追加完成记录。
- 墙里能上网的：`github.com`、`raw.githubusercontent.com` 等 GitHub 域名，查资料用 exa 那个 MCP 服务器（搜索、取网页正文）。别的网站直接取不到时用 exa 取，取不到就写“没取到”，不想别的办法绕墙。

## 用户定的范围（10-20 用户选“照这 5 条做”）
1. 提案本身：出处（作者、链接、日期、现在的状态是草案还是已定），序列的格式，能表达哪些状态，谁来发（agent 自己、包它的程序），终端怎么收。
2. 谁已经在用：Ghostty 自己、Claude Code、Codex、pi、omp、amp、agy 这些 agent 有没有发；别的终端或多路复用器（Zed、tmux、herdr 等）有没有收。
3. 和 paddock 的关系：paddock 内置终端收到后能做什么（窗格标题、侧栏状态、收件箱）；和 corral 给的状态是互补、重复还是会打架；corral attach 中间转不转这个序列。
4. 和 ranch／corral 的关系：corral 要不要发或转它，这类改动归 ranch，只写成建议，不在这件里改。
5. 许可：参考的实现若是 GPL（如 Zed 的应用层），只读思路不抄代码。

## 怎么算做完
- `docs/调研/P5-111-OSC7501调研.md` 逐条回答上面 5 条，带出处链接；最后给“做不做、先做哪一步”的建议，由用户定。
- 验证只做这些：`git diff --check`。觉得不够，在回复里说，不要自己加。

## 不要做
- 不改代码，不改 `docs/DESIGN.md`；不改 ranch、corral、Saddle。
- corral attach 转不转序列，看代码或文档判断就好，不起 paddock 窗口，不开 agent，不碰 `corral ls` 里现有的 agent，不对任何 agent `corral send`。
- 不抄 GPL 代码进文档（Zed 的 `terminal`、`terminal_view` 等应用层）：只写思路，必要时给链接。
- 不用 Python；命令里不用 `rm`、不用 `sh -c` 包长命令；前台命令别带等标准输入的东西。
- 不要按项目名或路径批量杀进程（`pkill -f paddock` 这类）。
- 不合并到 main，不推送。只在 `p5-111-osc7501` 上提交。

## 做完
在本文件末尾追加「## 完成记录」（在你的分支里提交）：做了什么、验证了什么、拿主意的地方、没做的事、踩过的坑，各几句话。踩过的坑指试了不行、换了办法的，工具或环境的行为和预想不一样的；没有就写「没有」，不要凑。回复里只写这几样，加上调研结论的三五句摘要和有没有要主控决定的事。命令都在前台跑完，全部做完后，回复最后一行写 DONE。
