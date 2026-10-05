# paddock：设计

状态：第一、第二阶段与迁移 M0、M1 均已完成，第三阶段这一批（P3-1 至 P3-10）已合并；paddock 已打包装到 `~/Applications`（2026-10-05）。本文记录已定决定、与 Saddle 的关系和待定问题；完整调研见 `docs/调研/T76-GPUI桌面化详细方案.md`（Saddle 原文的副本，作依据，不随本文更新）。来历和用户原话见 `docs/背景与决策记录.md`。

## 1. 是什么

用 GPUI 重做 Saddle 界面的独立桌面应用：在原生窗口里查看 corral agent、运行真实交互终端，不再依赖外层终端程序。终端体验以 Zed 内置终端为参照（用户已认可该水平）。

paddock 与 Saddle 在代码上完全分开：用到的 Saddle 代码迁入 paddock 自己维护，不再依赖 Saddle 的库；Saddle 的界面逐步用 GPUI 重做。Saddle 本身（含 TUI、插件、`saddle ctl`）继续独立使用，用户转到 paddock 开发后留作保底版（§3“Saddle 保底”）。全局只能有一份的运行时（corral、遥测、dispatch 及插件协议）移到独立仓库 ranch，Saddle 和 paddock 都只是前端，通过 ranch 装好的命令使用它（§3）。

## 2. 已定决定

| 决定 | 依据 |
| --- | --- |
| 评估 GPUI 桌面化，先做详细调研 | T76 登记与 10-05 用户指示 |
| 终端以“GPUI＋alacritty_terminal 自己画”为路线，目标是 Zed 内置终端的水平；暂不追完整嵌入 Ghostty | 用户：“其实zed的终端我试过，是可以满足我的要求的” |
| 先做只有一个窗口、一个终端窗格的原型，在隔离环境开发 | 用户 10-05 批准 |
| 独立仓库 paddock，原型搬入，上下文写进本仓库 | 用户 10-05 决定 |
| paddock 与 Saddle 互不影响，尤其是 Rust 依赖 | 用户：“即使合并到主分支，也要隔离起来，不能两边互相影响。尤其是所使用的rust库。” |
| 原型体验通过 | 用户 10-05：“我觉得是过了” |
| 要有主题系统，像 Saddle 那样：预置主题＋用户覆盖 | 用户 10-05：“起码有一套theme系统，就像saddle那样。”细节见 §8 |
| 第一阶段范围：一个窗口、左侧 Agents 列表、右侧一个终端窗格，加主题系统 | 用户 10-05：“第一阶段范围就按你的建议”。细节见 §9 |
| **代码与 Saddle 完全分开**：用到的 Saddle 代码迁入 paddock 自己维护，不再依赖 Saddle 的库；所有界面用 GPUI 重做 | 用户 10-05：“paddock和saddle是完全分开的，或者paddock要把用的saddle的文件都迁移过来，所有saddle的界面都用gpui重做”，“按这个方向调整”。细节见 §3、§10 |
| **paddock 要能单独分发**（别人不装 Saddle 也能用）：分发前把 corral 运行时及所需插件迁入 paddock、随 paddock 打包 | 用户 10-05：“要”（问：将来是否要让 paddock 能单独分发）。细节见 §3、§6 |
| **全局只能一份的运行时**：不再两边各留一份。顺序见 §3“步骤”。（归属原定 paddock，已由下面“独立成库 ranch”取代） | 用户 10-05：“saddle现在的corral或者drover这种全局只能有一个的。移到paddock中，之后saddle使用paddock的”，“分四步走”，“drover最后再搬”。细节见 §3 |
| **dispatch 迁完就全部转到 paddock 开发**；Drover 在全部切换完成后才迁，在那之前不用任务流程，口头布置、主控拆分委派 | 用户 10-05：“等dispatch迁移进去之后，我就打算全部在paddock中开发我的项目了。saddle就会退出”；“drover在全部切换完成前，不会迁移，我暂时不会使用任务workflow。而是口头布置任务，主控拆任务委派。” |
| **Saddle 不放弃，定位为保底版**（改了上一条的“Saddle 退出”）：不再给 Saddle 加新功能，只保证它和运行时对得上、不会坏；paddock 改 corral、遥测、dispatch 时优先兼容，不兼容时写需求给 Saddle 适配；Drover 迁走后 Saddle 没有任务界面，新插件、新功能只在 paddock 里有 | 用户 10-05：“saddle我暂时不放弃。遥测，dispatch也一样。”对保底版的定位：“我倒是同意这个”。细节见 §3 |
| **运行时独立成库 ranch，Saddle 和 paddock 都只是前端**（方案 3；改了“归 paddock”）：corral、遥测、dispatch 及插件协议放进新建的 ranch 仓库，由 paddock 主控兼管；两个前端只调用 ranch 装好的命令。前期由 paddock 负责把 corral 做到 Saddle 和 paddock 都能用。先把 corral 从 Saddle 拉出去，直到 Saddle 能和新的 corral 一起工作，再转回 paddock 开发。ranch 建 GitHub 公开仓库，不加许可证 | 用户 10-05：“其实我还是想，两个版本都保留。我其实能够接受corral/dispatch独立出来一个。然后saddle和paddock都依赖这个repo。”“选方案3，新建仓库，paddock主控兼管，名字你来选一个合适的。paddock前期负责把corral做到saddle和paddock都能用的地步。”“先把corral从saddle中拉出去，直到saddle能够和新的corral一块工作（打到现在的状态），咱们再转过头开发paddock。”“ranch建公开远程仓库。不加许可证。”细节见 §3 |
| **要有插件系统**（已取消，见下面“砍掉遥测、Drover、插件系统”）；插件界面走“乙”：进程、命令、生命周期沿用 Saddle 的插件协议，界面由插件描述、paddock 用 GPUI 原生画 | 用户 10-05：“还是做插件系统吧。这是一个应用的必备。”“按乙”。细节见 §3 |
| **不依赖 ratatui**（含插件 SDK 和仓库里维护的插件） | 用户 10-05：“不能依赖ratatui”。细节见 §4 |
| **`saddle ctl` 也迁移**，做成 `paddock ctl` | 用户 10-05：“这个也要迁移。” 细节见 §3 第 5 步 |
| **dispatch 剥离到 ranch（不带遥测），之后再开发 paddock；不要遥测和 Drover**：遥测、Drover 不迁，留在 Saddle 原样不动（不用即可），paddock 路线里的遥测查看页、Drover 暂不需要；插件 SDK 不迁；插件协议这一轮不迁（dispatch 做成 ranch 的命令，用不到它）。用户以后很可能只用 paddock | 用户 10-05：“我要等saddle中的都剥离完了，而且saddle运行和现在一样，再开发paddock”；“那就先不迁移插件sdk和drover。先把插件协议，dispatch以及遥测剥离过去”；看到 M3（遥测连同 dispatch）的规模后：“dispatch能拆出去吗？我现在都不想要遥测和drover了”；“dispatch我觉得要拆出去。这也算是基础功能。后期我极有可能只用paddock。因为tui维护起来还有ui什么的做起来太复杂了” |
| **砍掉遥测、Drover、插件系统，Saddle 和 paddock 同一起跑线**：Saddle 删掉整个插件系统（宿主、SDK、协议、Drover、Diff、dispatch 插件、示例插件、Plugins 页、`ctl plugin`）和遥测（存储、查看页、开关、`saddle telemetry`、`saddle agent`），对应测试改或删；数据留在磁盘上不删。paddock 路线同样拿掉插件宿主、插件界面、遥测查看页、Drover，以后真需要再重新设计。两个前端都只剩 Agents 面板、终端、设置、ctl，底下用 ranch 的 corral 和 dispatch。`saddle ctl` 保留（去掉 plugin 子命令），`paddock ctl` 照旧 | 用户 10-05：“我想砍saddle的功能……遥测，drover还有插件系统的远吗都干掉。这样dispatch剥离出去之后，saddle和paddock就都处于一样的状态了……遥测和drover也只是我臆想出来的功能……绑定很深的遥测，sdk还有插件以及drover什么的，只是我觉得有用罢了。发不成产品反倒不合适。”作者建议四点（数据留着、paddock 路线也砍、Diff 随插件系统去掉、`saddle ctl` 保留），用户：“可以，四点都按你的建议”；“对应的测试也要改或者删” |
| **Diff 插件不搬**；以后需要时在 paddock 里原生做（可能内置） | 用户 10-05：“diff插件我觉得不要搬了，diff是照顾tui而生的，现在没必要了，会开发更好的（可能会内置diff）。” |

## 3. 与 Saddle 的关系

- **代码**：paddock 不依赖 Saddle 的库（M0 起，见 §10）。迁入的代码在 paddock 里自己维护，文件开头注明来源（Saddle 提交 `df1c727` 的哪个文件）。之后两边各自演化：Saddle 的修复不会自动进入 paddock，需要时作为 paddock 的任务手动移植。
- **只共享公开约定**：paddock 与 Saddle 生态只通过下面这些公开约定打交道，不读 Saddle 的内部状态和配置文件：

  | 约定 | 归属 | paddock 怎么用 |
  | --- | --- | --- |
  | `corral` 命令（`ls`/`status`/`attach`/`start`/`stop` 等）及其 JSON 输出 | ranch（迁入前随 Saddle 安装） | 照公开格式调用；格式变化时 paddock 跟进 |
  | 插件协议 | ranch（迁入前在 Saddle） | paddock 做自己的插件系统：进程、命令、生命周期部分沿用这份协议，界面部分 paddock 自定（下文“插件”） |
  | 遥测命令（迁入前是 `saddle telemetry …`）、Drover 的公开命令 | 遥测归 ranch；Drover 在 Saddle（第 8 步再定） | 需要时照公开命令调用 |

- **运行时要求**：机器上要有 `corral` 命令，由 ranch 安装（`~/.local/bin/corral` → `~/.local/share/ranch/versions/…/bin/corral`，10-05 起）；paddock 不需要 Saddle TUI 在运行。
- **单独分发**（用户 10-05 定）：paddock 要能不装 Saddle 单独使用，所需的运行时随 paddock 打包（见下一条）。
- **全局只能一份的运行时归独立仓库 ranch**（用户 10-05 定，原话见 `docs/背景与决策记录.md` §6g、§6j）：corral、遥测、dispatch 的数据和全局位置（`~/.corral`、`~/.local/bin/corral`、技能目录、遥测数据库）每台机器、每个用户只能有一个主人。它们放进新建的 ranch 仓库（`../ranch`），由 paddock 主控兼管；Saddle 和 paddock 都只是前端，地位对等。
  - **依赖方式**：两个前端都只调用 ranch 装好的命令（PATH 上的 `corral`、遥测命令等），不在 Cargo 里引用 ranch 的 Rust 包，各自的依赖、锁文件和工具链互不进入（§4）。插件协议的类型怎么共享（各自照协议文档实现，还是引用 ranch 的协议包）到第 4 步定。
  - **安装**：由 ranch 负责：装进自己的不可变版本目录，链接 `~/.local/bin/corral`，并安装 corral 技能（以后还有 dispatch 技能）。Saddle 和 paddock 都不再打包、安装这些。单独分发 paddock 时，打包带上 ranch 的某个版本（§7）。
  - **ranch 自己的规矩**：独立的 AGENTS.md、DESIGN、任务文件和编译目录，规矩照 paddock（Rust stable、不用 Python、不依赖 ratatui、依赖精确管理）。
  - **Saddle 侧的改动**（不再打包安装 corral 和技能、插件和配置改用 ranch 装好的 corral、Updates 页、遥测命令等）写成需求交给用户/Saddle 主控，在 Saddle 自己的流程里做；paddock 主控不改 Saddle 仓库。
  - **步骤**（用户 10-05：先定“分四步走”，之后定要做插件系统、dispatch 迁完就转到 paddock、Drover 全部切换后再迁；Saddle 不退出，留作保底版）。切换前只做能直接搬的，要重做的界面都放到切换之后：
    1. 第三阶段这一批（§12），已完成。
    2. **corral 进 ranch**：新建 ranch 仓库，迁入 Saddle 的 `crates/corral-core`（独立包、不依赖 Saddle 其他代码）。冻结点取 Saddle `a31dea2`，不另与 Saddle 主控约定（用户 10-05：当时只开着 paddock 的主控）。迁入方式同 §10，命令、JSON 输出和 `~/.corral` 登记格式不变。切换只改 `~/.local/bin/corral` 的指向，已在运行的 agent 不做 upgrade，继续由原来那份管理（§7 第 9 条），用户在场时做。前期由 paddock 负责做到 Saddle 和 paddock 都能用（用户 10-05）：paddock 本来就按 PATH 调用，不用改；Saddle 默认用自己打包的 corral（`src/agent_program.rs`，插件也是），要 Saddle 主控改成用 ranch 装好的。Saddle 的 AGENTS.md 已加一句“corral 由 paddock 维护”（用户要求，Saddle 主控提交推送），要改成 ranch。任务见 `docs/任务/M2-ranch与corral.md`。
    3. **遥测进 ranch**：存储与命令（Saddle `src/telemetry/`，约 2600 行）自成一体、不依赖 Saddle 其他代码，也不用 ratatui，照 corral 的做法迁入 ranch（新增依赖 `rusqlite`，Saddle 精确固定 0.40.2、内含 SQLite），数据库格式（含版本号和升级规则）不变。ranch 提供与 `saddle telemetry`、`saddle agent` 用法相同的命令（如 `ranch telemetry`、`ranch agent`，名字到时定），在 PATH 上找得到；技能和 dispatch 改为调用它；Saddle 保留 `saddle telemetry`、`saddle agent` 转发过去（Saddle 侧需求）。
    4. **插件协议与 dispatch 进 ranch；paddock 做插件宿主底层**：协议（Saddle `crates/plugin-protocol`）归 ranch；paddock 照搬宿主底层（见下文“插件”）。dispatch（Saddle 内置插件，约 1700 行，没有界面：`route` 路由命令用 `TYPESAFE_API_KEY` 请求外部模型服务，依赖精确固定的 `ureq` 3.4.2；并安装 corral-dispatch 技能）迁入 ranch，作为 Saddle 和 paddock 都能加载的插件：路由规则、请求格式、技能文件内容不变，只把技能里写的 `saddle …` 命令换成 ranch 的。技能目录同一时间只能放一个版本，从此归 ranch 安装和管理。
    - **切换**：第 4 步完成后，用户转到 paddock 开发；Saddle 留作保底版（用户 10-05，见下文“Saddle 保底”）。
    5. **`paddock ctl`**（用户 10-05：“这个也要迁移”）：Saddle 的 `saddle ctl` 让 agent 和脚本在运行中的界面里开 shell、显示或新建 agent、安排标签页和四向分屏、关闭显示，并查询结果。传输层（Saddle `src/control.rs`，418 行：每个实例一个私有 Unix 套接字，JSON 消息，有长度、超时和队列上限）和命令行客户端（`src/control_cli.rs`，253 行）照搬；界面端（`src/app_control.rs`，632 行）对着 paddock 的窗口和布局重写，复用 P3-4 的“新建 agent 再在指定位置打开”。paddock 用自己的运行目录（如 `$XDG_RUNTIME_DIR/paddock`）和自己的环境变量（如 `PADDOCK_INSTANCE`、`PADDOCK_PANE`，注入 paddock 开的 shell），不碰 Saddle 的；配套的技能（Saddle `skills/saddle/SKILL.md`）改写为 paddock 版。`ctl plugin`（调用插件方法）等插件系统完成后再跟上。
    6. ~~**遥测查看页**~~（已取消，用户 10-05 砍掉遥测）：Saddle `src/telemetry_view.rs`（约 3100 行，ratatui）用 GPUI 重做。
    7. ~~**插件界面（乙）**~~（已取消，用户 10-05 砍掉插件系统）：先设计描述界面的协议给用户看，再做显示层、插件启动器、设置里的 Plugins 页。
    8. ~~**Drover**~~（已取消，用户 10-05 砍掉 Drover）：全部切换完成后才迁（用户 10-05）；在那之前用户不用任务流程，口头布置任务、由主控拆分委派。它的界面用 ratatui 写，按“不依赖 ratatui”必须重写，成为新插件界面的第一个用户。数据核心怎么放未定，到这一步给用户看方案：A 整个仍是插件，移到 ranch 或 paddock 维护；B 拆出数据与派发核心、对外提供接口，界面原生；C 仍是插件但只提供数据、界面由 paddock 画。
- **搬迁完成前的兼容规矩**：在遥测、dispatch 等完成搬迁之前（corral 已于 10-05 搬到 ranch），正式的一份仍在 Saddle，paddock 照公开命令调用，不改它们的格式。
- **Saddle 保底**（用户 10-05）：Saddle（TUI）不放弃，留作退路（GPUI 仍是 pre-1.0；只有终端、经 SSH 的环境）。不再给它加新功能，只保证它和运行时对得上。
  - Saddle 和 paddock 用的是 ranch 装好的同一份运行时。Saddle 改完之前（它默认用自己打包的 corral，插件也是），会有新旧两份 corral 同时操作同一批 agent（`~/.corral` 共用），这段时间兼容要双向：ranch 的 corral 开的 agent，Saddle 那份能看、能接入、能停，反过来也一样。
  - 改 ranch 时，任务文件写明对两个前端的影响：只改内部实现的，前端不用动；公开约定有兼容的新增，写说明交用户决定 Saddle 跟不跟；不兼容的先尽量改成兼容，做不到就先写好 Saddle 怎么适配，交用户/Saddle 主控，定好次序再合并 ranch。Saddle 的适配在 Saddle 自己的流程里做，paddock 主控不改 Saddle 代码。
  - 事先接受：Drover 迁走后 Saddle 没有任务界面；新插件、新功能只在 paddock 里有。
- **不改 Saddle**：不修改 Saddle 仓库。原先打算请 Saddle 新增的接口（§5 的缺口）改由 paddock 在自己的代码里解决；只有公开约定本身要变时，才写成需求交给用户/Saddle 主控。
- **Drover 并存**：Drover 每个用户只允许一个插件进程持有数据：在 paddock 迁入（上面第 8 步）之前，用户不使用它；迁入后由 paddock 持有数据。
- **插件**（已取消：用户 10-05 砍掉插件系统，Saddle 也删掉；下面留作记录）（Saddle `df1c727` 的插件系统设计）：
  - 插件是独立程序（目录里有 `plugin.toml` 和可执行文件），宿主启动它、按插件协议通信；插件自己用 ratatui 画字符画面，以结构化格子数据发给宿主显示。业务数据归插件自己管，宿主不碰。（以上是 Saddle 的做法；paddock 沿用进程和生命周期部分，界面部分另定，见下。）
  - 各插件的数据：Drover 有（`~/.drover/projects`、各项目的 `queue.md`、`tasks.state` 等），且每个用户的数据只允许一个 Drover 进程持有，第二个直接启动失败；Drover 记遥测要通过宿主调用 `saddle telemetry`，没有宿主路径时退回不记遥测、直接 `corral send`。Diff 读 git，dispatch 一问一答，都没有共享数据。
  - **paddock 的做法**（用户 10-05 定：要有插件系统；界面走“乙”；不依赖 ratatui）：
    - **照搬**：宿主底层（启动和管理插件进程、按协议通信、插件登记、资源安装、命令行入口，Saddle `src/plugins/` 中约 3500 行），不用 ratatui；协议包（`crates/plugin-protocol`，约 420 行）归 ranch（第 4 步）。paddock 用自己的插件登记（如 `~/.config/paddock/plugins.toml`），不读 Saddle 的。
    - **界面（乙）**：不再由插件画字符画面。插件只描述界面里有什么（列表、文字、按钮、表单等），paddock 用 GPUI 原生画，和 paddock 其余部分一个样子。这部分协议由 paddock 新定，先写设计给用户看。Saddle 现有插件的界面不能直接在 paddock 里显示，它们没有界面的命令照样可用。
    - **SDK**：不搬 Saddle 的 SDK（它带 ratatui）。paddock 提供自己的 SDK，只有协议和描述界面的类型，不带任何画图库。
    - Saddle 用 ratatui 画的插件显示层、插件启动器、设置里的 Plugins 页（约 2000 行）用 GPUI 重做。
  - **各插件**：Diff 不搬（用户 10-05），以后需要时在 paddock 里原生做（可能内置）；dispatch 按第 4 步，归 ranch，作为两个前端都能加载的插件；Drover 按第 8 步。
- **遥测**（Saddle `df1c727` 的 `docs/遥测使用.md`，用户 10-05 要求核查）：
  - 数据在 `~/.local/state/saddle/telemetry/`（SQLite `telemetry.sqlite3`，带格式版本号，现为 2；正文在 `blobs/`），默认关闭。写入方首次写入会把旧格式自动升级。
  - 使用者：`saddle telemetry` 命令、`saddle agent`、Drover（经宿主程序 `SADDLE_HOST_BIN` 调用前两者）、corral-dispatch 技能、Saddle TUI 的 Settings 总开关和遥测查看页。
  - 与 corral 同类：数据只有一份。搬迁（上面第 3 步）之前 paddock 不碰数据库，只调用已安装的 `saddle telemetry` 命令（JSON 输入输出）；搬迁时提供与 `saddle telemetry`、`saddle agent` 用法兼容的命令，否则技能和 Drover 的记录会失效。
- **许可**：Saddle 仓库目前没有许可证文件。代码属于用户本人，迁入没有问题；paddock 公开发布前，需要先为两边定好许可（§7）。

## 4. 依赖与工具链隔离

- 每个 Cargo 清单有自己的 `[workspace]` 和 `Cargo.lock`。
- 不再有 `saddle` 依赖（M0；锁文件因此少了 32 个包，其余版本未变）。`alacritty_terminal`、`portable-pty` 等库的版本由 paddock 自己决定，不再要求与 Saddle 一致；升级照常作为单独任务。`ratatui`、`crossterm` 已在 M1 去掉（锁文件又少了 68 个包），按键、鼠标、颜色改用 paddock 自己的类型。
- **不依赖 ratatui**（用户 10-05：“不能依赖ratatui”）：应用、插件 SDK 和仓库里维护的插件都不用 ratatui（也不用 crossterm）。别人写的插件程序在它自己的进程里用什么库，不算 paddock 的依赖。
- 编译目录：`$HOME/Developer/personal_projs/paddock-worktrees/.target/<子目录>`，不与 Saddle 共用；每个工作目录一个子目录（主仓库 `main`、任务 worktree 用分支名、审查用 `review`），避免并行 worktree 互相覆盖同名包的产物（用户 10-05 同意）。
- GPUI：`gpui-pre =0.3.8` / `gpui-pre-platform =0.3.8`（zed@279fe07 的第三方快照，发布者 huacnlee，Apache-2.0），特性 `font-kit`、`runtime_shaders`。所有 `gpui-pre-*` 一起精确固定、一起升级。
- `runtime_shaders`：本机 Xcode 27 缺 Metal 工具链组件；安装属于系统安装，未做，改为运行时编译着色器。
- 工具链：本机默认 stable `rustc 1.96.0` 可以编过；暂未加 `rust-toolchain.toml`。写精确版本会让 rustup 另装一份同版本工具链，等 GPUI 要求更新的 Rust 时再与用户确定。

## 5. 原型结论（2026-10-05）

详见 `docs/原型实测记录.md`。

- **已验证**：
  - 合成样例渲染：中英混排、表情、表格与框线对齐、样式、颜色。
  - 窗口尺寸与 PTY 行列协商。
  - 交互 shell 启动与关闭。
  - 真实 Claude Code 接入：显示正确；关闭窗口后 agent 继续运行。
  - 大量输出不拖慢，约 100–120 帧/秒。
  - 用户实际体验：在 Claude Code 里中文输入和显示正常（10-05）。
- **原型发现的缺口**（原打算请 Saddle 新增接口；迁移后由 paddock 在自己的代码里解决，按需排期）：
  1. 没有“有新输出”的通知，原型每 8 ms 轮询。
  2. ~~网格读取与默认配色绑定 ratatui~~：paddock 只用自己的 `rows`、`palette`；M0 未迁 Saddle 的 ratatui 绘制部分。
  3. ~~输入编码函数参数是 crossterm/ratatui 类型~~：M1 已换成 paddock 自己的类型。
  4. `Session::spawn` 不能控制环境变量。用户体验时实际遇到：从 Claude Code 会话里启动原型，窗格中的 Claude Code 继承了 `CLAUDECODE`、`CLAUDE_CODE_CHILD_SESSION`、`CLAUDE_CODE_MESSAGING_*` 等变量，会话记录被关闭。T1 起 paddock 启动时按名单清理 17 个身份变量。
  5. 写入通道容量 64，可能在极大粘贴时返回 busy。
  6. 程序向终端查询颜色时，应答的是 xterm 默认值，不是 paddock 主题的调色板。
- **GPUI 方面的发现**：
  - 单独排版的宽字符上自带删除线不显示，原型自己画。
  - 窗口被完全遮挡时暂停绘制。

## 6. 路线

- **已定**：所有界面用 GPUI 重做（§2）。先做迁移（§10），再逐步补界面。
- **建议的顺序（未批准）**：
  1. 迁移：去掉 `saddle` 依赖，行为不变（M0，已完成）；再去掉 `ratatui`、`crossterm`（M1，已完成）。
  2. 第二阶段（已定，见 §11）：Agents 面板按 Saddle 做全、标签页和分屏、宠物、菜单栏和 `.app` 打包；外观不比 Saddle 差。
  3. 第三阶段：插件（先 Drover）、Attention、新建 agent、Settings 页、历史搜索、布局保存等。
  4. 全局只能一份的运行时移到独立仓库 ranch，并做插件系统（已定，§3“步骤”）：corral → 遥测的存储与命令 → 插件协议与 dispatch（进 ranch）、paddock 的插件宿主底层 → 切换到 paddock（Saddle 留作保底版）→ `paddock ctl` → 遥测查看页 → 插件界面 → Drover；单独分发的打包（签名公证等）见 §7。
- ~~**插件界面已定走“乙”**~~：插件系统已取消（用户 10-05）。

## 7. 待定问题

1. ~~原型体验是否达到要求~~：已通过（10-05）；第一阶段已完成，之后的阶段范围待定（§6）。
2. 生产用 GPUI 依赖渠道：固定官方仓库提交，还是继续 `gpui-pre` 快照。
3. pre-1.0 的 GPUI 是否符合“只用成熟、活跃维护的库”；是否接受工具链跟随最新稳定版 Rust。
4. ~~迁移路线~~：已定为全部用 GPUI 重做（10-05）。仍待定：是否引入 gpui-component；首期是否只做 macOS。
5. ~~插件界面~~：插件系统已取消（用户 10-05）。
6. ~~paddock 与 Saddle TUI 同时运行时 Drover 的持有权~~：已定由 paddock 持有（§3 第 8 步，全部切换后）。布局文件各用各的（§12 P3-9）。
7. 许可（paddock 和 Saddle 都还没有）、应用名与标识、签名公证，以及何时建远程仓库。Clawd 是 Claude Code 的吉祥物形象：用户 10-05 选定公开仓库照原样带着它（“照原样公开”）；猫和卡皮巴拉是 Saddle 原创。远程仓库已建（10-05，`github.com/firegnu/paddock`，public）；不加许可证（用户 10-05：“不加许可证。”），即保留所有权利。
8. paddock 的任务是否纳入 Saddle 的 Tasks（Drover）管理。
9. ~~corral 切换的具体做法~~：已定（用户 10-05）：只改 `~/.local/bin/corral` 的指向，已在运行的 agent 不做 `corral upgrade`，继续由启动它的那份管理；用户在场时做（§3 第 2 步）。切换后 Saddle 完成剥离并部署，用户要求再升级现有会话：先用测试 agent 演练，`saddle/main`、`paddock/main` 已用 `corral upgrade` 接到 ranch 的 corral（10-05）。
10. ~~`saddle ctl` 是否迁移~~：要迁（用户 10-05），做成 `paddock ctl`，排在切换之后第一件（§3 第 5 步）；用户的工作方式在切换时用不到它。
11. ~~ranch 的远程仓库和许可~~：GitHub 公开仓库，不加许可证（用户 10-05）。

## 8. 主题

用户 10-05 定下（原话和选项见 `docs/背景与决策记录.md` §6）：

- **预置主题沿用 Saddle 的 Dune、Tide、Lagoon**。原先从 Saddle 公开接口读取界面色；用户 10-05 定为与 Saddle 分离，随迁移（§10）把 Saddle `df1c727` 中这三套的色值复制进 paddock，注明来源，此后独立维护，不再自动跟随 Saddle 的主题改动。
- **每套主题自带终端调色板**：16 个基本色，以及终端默认字色、底色、光标、选区。Saddle 的这些颜色由外层终端决定，paddock 没有外层终端，所以要自己定。Saddle 主题里的 `Reset`/默认值也要在 paddock 里换成具体颜色。
- **配置**：paddock 自己的配置文件，写法照 Saddle：`theme = "…"` 选预置主题，`[colors]` 覆盖单个颜色，最终颜色＝预置＋覆盖。不读写 Saddle 的配置文件。
- **第一阶段**只用配置文件，改完重启生效；不做设置页。
- **调色板**（用户 10-05 看截图后批准）：Dune 取 Gruvbox dark，Tide 取 Nord，Lagoon 取 Everforest dark（均 MIT，出处写在 `app/src/theme.rs`）；除 0 号外 15 色在底色上对比度 ≥3:1（有测试）。终端配色的 `[colors]` 键为 `terminal_*`，见 `app/README.md`。
- **terminal 主题**：不提供，写了报错。
- **字体**（T4）：`font`、`font_fallbacks`、`font_size`、`line_height` 写在同一配置文件；命令行参数优先。

## 9. 第一阶段（已完成）

用户 10-05 批准（原话见 `docs/背景与决策记录.md` §6）：

- 一个窗口：左侧 Agents 列表，右侧一个终端窗格。
- 点哪个 agent 就接入哪个（`corral attach`），也能开普通 shell。
- §8 的主题系统。
- 不做：分屏、标签、插件页面、Drover、布局保存、设置页。

第一批任务的划分（用户 10-05 同意）：

- 代码位置：原型提升为 `app/`，不另起新代码。原型 README 改存为 `docs/原型实测记录.md`。
- 侧栏：每个 agent 一行，显示状态色点、名字、状态（精简版）。以后逐步加强度、会话标题、分支和改动数，每样单独做；所以每一行做成独立组件，侧栏宽度可在配置中设置。（第二阶段改为直接按 Saddle 做全，见 §11。）
- 任务：T1 应用骨架、T2 主题系统、T3 Agents 侧栏、T4 字体配置，均已合并。任务文件见 `docs/任务/P1-T*.md`。

## 10. 迁移：去掉 `saddle` 依赖

目标：paddock 不再依赖 Saddle 的库，行为不变。

- **迁入范围**：paddock 现在用到的 Saddle 模块（Saddle `df1c727`，约 2200 行）：

  | Saddle 文件 | paddock 用它做什么 |
  | --- | --- |
  | `pty.rs`、`terminal.rs` | PTY 会话、alacritty 解析与终端查询应答 |
  | `viewer.rs` | shell 与 `corral attach` 的生命周期 |
  | `input.rs` | 按键、鼠标、粘贴编码 |
  | `corral.rs`、`agents.rs` | 取 agent 列表，排序、分组、判断状态 |
  | `theme.rs` | Dune、Tide、Lagoon 的界面色和颜色写法解析 |

  这些文件还引用了少量 Saddle 内部代码（`command::run`、`history::History`、`git` 的摘要类型、`layout_state::Content`）。只迁 paddock 实际走到的部分；只服务 TUI 的部分（如 `Screen::render` 写 ratatui 缓冲区、历史模式）不迁。
- **原则**：
  - 迁移本身不改行为，现有测试照样通过；Saddle 里对应模块的测试一并迁入。
  - 每个迁入文件开头注明来源文件和提交号。
  - 分两步（用户 10-05 定）：第一步只迁移，`ratatui`、`crossterm` 类型照原样保留；第二步单独任务，换成 paddock 自己的类型，去掉这两个依赖。
- **之后**：AGENTS.md 中“只通过 git 依赖按固定提交号引用 Saddle”“升级 Saddle 引用”“与 Saddle 交换类型的库同版本”等规矩已在 M0 改写为 §3 的关系。
- **M0 结果**（10-05）：迁入 `pty`、`terminal`、`viewer`、`input`、`corral`（含 `command::run`）、`agents`、`preset`（原 `theme`）；Saddle 对应测试迁入 `app/tests/`，原 Python 假程序改为 shell 脚本。`agents::Panel` 中只服务 TUI 键盘操作的字段和方法暂时保留（`absorb` 依赖它们、迁入的测试覆盖它们），留待界面重做时再清理。
- **M1 结果**（10-05）：`input.rs` 定义 paddock 自己的按键、修饰键、鼠标事件和区域类型，`preset.rs` 定义自己的 `Color`，取值与原类型一致；删去 `ratatui`、`crossterm`，锁文件少了 68 个包，无新增。编码测试只改输入的构造写法，期望字节未改。

## 11. 第二阶段（已完成，10-05）

用户 10-05 定下（原话见 `docs/背景与决策记录.md` §6d）：

- **外观不比 Saddle 差**：以用户日常 Saddle 截图（10-05，未入库）和 Saddle `df1c727` 的实际实现为准，规格参考 Saddle `docs/设计稿/agents-panel-3a`。界面文字沿用 Saddle 的英文标签。
- **范围与顺序**（每件先写任务文件、做出截图给用户看，认可后合并）：
  1. P2-1 Agents 面板按 Saddle 做全：头部计数、分组标题、每个 agent 的多行信息（状态点与动画、哪家、强度格、状态、时间、会话标题、当前活动、git 分支与改动、目录、实例号与接入数与来源）、选中样式、折叠与排序。需迁入 Saddle 的 git 摘要代码，做法同 §10。底栏这次只做显示、折叠、排序；New、Stop 属于操作 agent，留到第三阶段。 **已完成（10-05，用户认可）**：原生卡片，信息与 Saddle 一致；不放 TUI 的 `[Attached]` 焦点提示；强度用三格信号图标；侧栏默认宽度改为 380 pt。用户补充的要求：“不一定复刻，但是信息不能少，而且我觉得可以做的更加漂亮”。
  2. P2-2 标签页与分屏：外观照 Saddle 的圆角标签和窗格边框（标题、边框上的按钮）；切走时 shell 不再结束。 **已完成（10-05，用户认可）**：按钮放在窗格标题栏右侧，弹框居中；活动窗格是运行中的 shell 时，侧栏点 agent 新开标签页；未做拖动调整大小。
  3. P2-3 宠物：Clawd、猫、卡皮巴拉，在标签栏右侧空地巡游；迁入 Saddle 的宠物逻辑和图片素材，用 GPUI 直接画像素图。配置照 Saddle 的 `mascot_enabled`、`mascot`；不要 Saddle 的“显示方式”（方块/图片）选项。用户 10-05 补充：只用图片版，不要方块字符版（“如果支持图片的话，就不需要再有兼容像素的画法了”）。 **已完成（10-05，用户认可）**：像素整 2 倍绘制，标签条加高到 48 pt。
  4. P2-4 菜单栏与 `.app` 打包。用户 10-05 选定：快捷键照 macOS 常见约定（⌘T、⌘N、⌘D、⇧⌘D、⌘W、⇧⌘W、⇧⌘[ ]、⌘1–9）；图标用猫的像素图；打好的 app 放 `~/Applications`；不签名、不公证。 **已完成（10-05，用户同意合并并安装）**：已装到 `~/Applications/paddock.app`；从桌面启动时向登录 shell 取 `PATH`，新 shell 开在主目录。

## 12. 第三阶段（这一批已完成，10-05，待用户体验）

用户 10-05 定下（原话见 `docs/背景与决策记录.md` §6e、§6f、§6g）：继续把 Saddle 的组件搬过来；不涉及 corral 运行时、遥测、Drover 这类共享运行时绑定的，一次做完，不必每件等用户回复。

### 窗口与对话框按 macOS 做

用户 10-05：不必复刻 TUI 的 Esc 关闭，GPUI 有窗口。TUI 只有一块屏幕，设置页、表单只能盖在上面、按 Esc 退出；paddock 改用 macOS 的做法：

- **独立窗口**：设置、新建 agent、About 各是一个单独的窗口，用红点或 ⌘W 关闭；已开着时再次打开就提到最前。
- **系统提示框**：需要确认的事（停止 agent、关闭运行中的 shell、关掉有未保存改动的设置窗口）用系统原生提示框（GPUI `window.prompt`），不自己画弹层。
- **系统选文件夹对话框**：选目录时用（GPUI `prompt_for_paths`）。
- **仍可按 Esc 收起的**：性质上是菜单或浮动面板的东西——新标签、分屏时弹出的选择，Attention 列表，跳转搜索，终端里的查找栏。点外面同样收起。

### 范围与顺序

每件照例写任务文件、单独分支、自查后合并；用户 10-05 改为不逐件等回复，全部做完后一并汇报截图，并重新安装 `~/Applications/paddock.app`。

1. **P3-1 Settings** **已完成（10-05，用户认可）**：配置新增 `refresh_ms`、`corral`。
2. **P3-2 设置与 About 改为独立窗口**：**已完成（10-05）**。⌘, 打开设置窗口。保留草稿和 Save（用户 10-05 选定）：有未保存的改动时关窗，弹系统提示框 Save / Don't Save / Cancel；Save 失败（写错的值、文件被外部改过）时窗口不关，留在原处显示原因。退出程序时同样先问。About 改为小窗口。
3. **P3-3 关闭运行中的 shell 先确认**（Saddle 有，paddock 还没有）：**已完成（10-05）**。关窗格、关标签页、关主窗口、退出时，若会结束运行中的 shell，弹系统提示框列出这些 shell，Cancel 什么都不关；agent 窗格只是断开，不问。
4. **P3-4 新建、停止 agent**：**已完成（10-05）**；前缀默认取项目目录名。只调用公开的 `corral start`、`corral stop`（用户 10-05 同意算在这一批）。
   - 新建窗口照 Saddle 的信息：项目目录（从现有 agent 的工作目录里选，或用系统选文件夹对话框）、名字（自动建议，可改）、Claude 或 Codex、第一条消息（可选）、在哪里打开（当前窗格、新标签页、四个分屏方向）；Advanced 里是完整命令和将要执行的调用预览。失败时窗口不关、填的内容保留。
   - 停止：选中 agent 后菜单或侧栏按钮，系统提示框确认后调用 `corral stop`。
   - 开发测试只用假 corral；实测只开 `paddock/test-*`，用完停掉。
5. **P3-5 Attention**：**已完成（10-05）**；顺带修正了排第一的 agent 的新回复会被误清。等待中、出错的 agent 和有新回复（未看过的一轮结束）的 agent 列成一张清单；侧栏头部显示数量，点开是浮动列表，点一行就打开该 agent，不替它回答。插件来源随 §3 第 4 步再做。
6. **P3-6 跳转搜索**：**已完成（10-05）**，⌘P。按项目或名字过滤 agent，回车打开（已在某个窗格里就跳过去）；也能搜到设置的各页。浮动面板。
7. **P3-7 终端里查找**：**已完成（10-05）**。⌘F 在当前窗格顶部打开查找栏，在回看历史里搜索，⌘G / ⇧⌘G 下一处、上一处；照 Saddle 的历史查找逻辑迁入。
8. **P3-8 窗格放大**：**已完成（10-05）**，⇧⌘↩。有多个窗格时把当前窗格临时铺满终端区，再按一次还原（Saddle 的 Zoom / Restore）。
9. **P3-9 布局保存与恢复**：**已完成（10-05）**；带 `--attach` 或 `-- 程序` 启动时不恢复也不覆盖存档。退出时把标签页、分屏和每个窗格的内容（shell 及其目录、agent 名字）存进 paddock 自己的状态文件，下次启动恢复；shell 在原目录重新开，agent 重新接入，已不存在的 agent 显示说明。不读写 Saddle 的布局文件。
10. **P3-10 Diagnostics**（用户 10-05 加入）：**已完成（10-05）**。设置窗口里的只读一页，显示 paddock 用的命令（corral、git、登录 shell）及其解析到的路径，最近一次读 agent 列表、读配置、保存和恢复布局的结果；打开该页或点 Refresh 时检查，只在内存里保留最近一次结果。

菜单新增 **Agent**：New Agent…（⇧⌘N）、Stop Agent…、Go to Agent…（⌘P）、Attention…（⇧⌘A）。Edit 加 Find…（⌘F）、Find Next（⌘G）、Find Previous（⇧⌘G）。View 加 Zoom Pane（⇧⌘↩）。快捷键都照 macOS 常见约定，和终端里程序常用的 Ctrl 键不冲突。

### 这一批不做

- 遥测、插件系统、dispatch、Drover（Tasks）：按 §3“步骤”做。
- Saddle 的 `saddle ctl` 控制接口（agent 用它开窗格）：做成 `paddock ctl`，按 §3“步骤”第 5 步做。
- Saddle 设置里的 Plugins 页（等插件宿主）、Updates 页（Saddle 的检查的是 Saddle 和 corral 的安装记录；paddock 自己的更新检查以后另定）。
