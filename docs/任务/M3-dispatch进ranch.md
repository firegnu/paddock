# 任务 M3：dispatch 从 Saddle 剥离到 ranch（不带遥测）

2026-10-05 起草，paddock/main（兼管 ranch）做 ranch 部分；Saddle 部分写需求交 Saddle 主控。待用户看过再动手。
类型：迁移＋切换安装
依据：用户 10-05：“dispatch我觉得要拆出去。这也算是基础功能。后期我极有可能只用paddock。”“我现在都不想要遥测和drover了”。paddock DESIGN §2；`docs/背景与决策记录.md` §6k。

## 现状（10-05 只读核对，Saddle main `c21674a`）

- dispatch 是编译进 saddle 的内置插件（`plugins/dispatch`，依赖 `crates/core-plugin`），只有一个命令：`saddle plugin run dispatch route`。从标准输入读任务摘要，用 `TYPESAFE_API_KEY` 请求 TypeSafe 的分类模型（`https://api.typesafe.ai/v1/systemone`），输出路由建议；没有 key 或调不通时由主控自己判断。
  - 代码约 800 行（`route.rs`、`rules.rs`、`transport.rs`、`json.rs`），单元测试 344 行（纯计算，不联网，不用 Python）。依赖 `ureq =3.4.2`（rustls）、`serde`、`serde_json`、`sha2`。
  - 路由时由 Saddle 宿主在进程内记遥测（只有带 `--record-context` 时）。
- `corral-dispatch` 技能（`SKILL.md`、`README.md`、`遥测操作.md`、`项目AGENTS模板.md`）由 Saddle 嵌入，安装到 `~/.claude/skills/corral-dispatch`、`~/.agents/skills/corral-dispatch`；Saddle 用归属记录（`~/.local/state/saddle/plugin-resources.json`）管着这些文件。

## 要做的

### 一、ranch（paddock 主控做）

1. 迁入路由代码到 `crates/dispatch`，做成程序 `ranch`，命令 `ranch dispatch route`：标准输入、输出、`TYPESAFE_API_KEY`、路由规则和请求格式不变；**去掉遥测**（不再有 `--record-context`、`--brief-file`）。照 M2：先原样复制一次提交，再改；Rust 文件开头注明来源。
2. 技能：迁入 `corral-dispatch`，
   - 删掉 `遥测操作.md`，`SKILL.md`、`README.md` 里讲遥测和 `--record-context` 的段落删掉；
   - 命令 `saddle plugin run dispatch route` 改成 `ranch dispatch route`；
   - 其余内容不变。
3. `ranch dispatch install-skills`：安装技能，照 corral 的 `install-skills` 做法（征得同意、只覆盖自己装的、遇到链接或别人的文件不动并说明）。
4. 迁入路由的单元测试；打包工具把 `bin/ranch` 放进版本目录。检查：`cargo test --all-targets`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`。

### 二、Saddle（写需求，交 saddle/main）

1. 删掉内置 dispatch 插件（`plugins/dispatch`；`crates/core-plugin`、`src/plugins/capture.rs` 等只为它服务的部分一并删，由 Saddle 主控判断），去掉 `saddle plugin run dispatch route`。
2. 不再安装 `corral-dispatch` 技能，并从归属记录里放掉它（文件留着，交给 ranch 接管）。
3. 遥测、Drover、插件 SDK 不动。Saddle 的 Plugins 页不再列出 dispatch。

### 三、切换（用户在场）

1. 生成 ranch 版本目录，链接 `~/.local/bin/ranch`。
2. Saddle 部署新版、用户重启 Saddle。
3. `ranch dispatch install-skills`：先给用户看新旧技能的差别（只差遥测部分和命令名），同意后安装。
4. 核对：`ranch dispatch route` 用一段测试摘要跑一次（有 key 时看到路由建议，没 key 时看到回退说明）；Saddle 照常运行；paddock 不受影响。

## 怎么算做完

- “dispatch我觉得要拆出去”（用户 10-05）。
- 遥测和 Drover 不迁（用户 10-05：“我现在都不想要遥测和drover了”）。

## 要用户定的

1. 程序名 `ranch`、链接 `~/.local/bin/ranch`，命令 `ranch dispatch route`。
2. 技能里删掉遥测部分（`遥测操作.md` 和相关段落）。
3. Saddle 那边直接去掉 `saddle plugin run dispatch route`（技能不再用它），不留转发。

## 不要做

- 不改路由规则、请求格式和输出。
- 不迁遥测、Drover、插件 SDK、插件协议。
- 不改 Saddle 仓库（由 Saddle 主控做）；不对用户的 agent 做写操作。

## 进度记录

2026-10-05，paddock/main。用户同意三件事（`ranch dispatch route`、技能删遥测部分、Saddle 不留转发），并要求“该交给saddle就交给saddle”。

- **第一节完成**：ranch 先原样复制 Saddle `c21674a` 的 `plugins/dispatch` 和 `crates/core-plugin`（`e808979`），再改（`d771c10`）：
  - 去掉插件外壳和遥测采集，路由规则、请求、重试、输出不变（保留了只为采集加的“解析后超 16 MiB 失败”检查，免得同样的回复结果不同）。
  - 测试保留全部业务断言；原来检查采集内容（begin/end）的地方改为检查 stdout 的错误说明。新增 4 项安装技能的测试（临时 HOME）。
  - 技能删 `遥测操作.md` 和讲遥测的段落，路由命令改成 `ranch dispatch route`，不可用原因改为看退出码和 stdout；`项目AGENTS模板.md` 逐字节不变；README 改写安装说明。
  - `ranch dispatch install-skills`：照 corral 的做法（确认、`--dry-run`、软链接不动），目录里不属于技能的文件（如旧的 `遥测操作.md`）只提示不删。
  - 打包工具加 `bin/ranch`；`bin/corral` 校验和不变。
  - 锁文件以 Saddle `c21674a` 的为起点，corral 原有版本不变，只多了两个本地包。
  - 测试共 63 项通过，clippy、fmt 通过。打包后的 `ranch` 在临时 HOME 里试过：帮助、没 key 时退出码 1、用法错误退出码 2、dry-run、不确认不写、`--yes` 写入。没有发真实路由请求（花钱，留到切换时问用户）。
  - 测试夹具 `tls-key.pem` 与 ureq 3.4.2 发行包的 `testdata/key.pem` 逐字节相同，`.gitleaks.toml` 只放行这一个路径；gitleaks、trufflehog 查后推送（ranch `d771c10`）。
  - 版本目录 `~/.local/share/ranch/versions/d771c10/` 已生成，未链接。
- **第二节**：需求已用 `corral send` 交给 `saddle/main`（10-05），等它删插件、放掉技能归属、改 AGENTS.md、部署。
- **范围扩大**（用户 10-05，见背景记录 §6l）：Saddle 再砍掉整个插件系统和遥测，对应测试改或删，数据留在磁盘上。补充用 `corral send --after saddle/main` 挂上，在它这一轮结束后送达。本任务第三节（链接 ranch、装技能、核对路由）不变；核对时加上 Saddle 照常可用（Agents 面板、终端、设置、布局恢复、`saddle ctl`）。
- **标签**（用户 10-05 同意）：请 saddle/main 在删除前的 `c21674a` 打带说明的标签 `before-cut` 并推送，被删功能的代码以后可从它取回。同样用 `corral send --after` 挂上。
