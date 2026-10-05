# 任务 P3-10：Diagnostics

2026-10-05，paddock/main 自己做。
类型：功能变更
依据：DESIGN §12 第 10 项（用户 10-05 加入）：设置窗口里的只读一页，显示 paddock 用的命令（corral、git、登录 shell）及其解析到的路径，最近一次读 agent 列表、读配置、保存和恢复布局的结果；打开该页或点 Refresh 时检查，只在内存里保留最近一次结果。参照 Saddle（`df1c727` 的 `src/diagnostics.rs`）。不逐件等回复。

## 要做的

1. 设置窗口加 Diagnostics 页（也能从 Go to Agent 搜到），只读：
   - 命令：corral、git、shell 各自按启动时的方式在 PATH 上找到的路径（找不到时说明）；corral 用公开的 `corral --version` 显示版本，git 用 `git --version`。
   - Agents：最近一次 `corral ls` 的时间和结果（几个 agent，或错误）。
   - 配置：文件路径；启动时是读的文件还是没有文件用了默认值；现在读一遍能不能用。
   - 布局：存档路径；启动时恢复了、没有存档，还是读不出来（原因）；最近一次保存的时间和结果；存档因读不出来被保留、这次不保存时说明。
   - 启动：从桌面还是终端启动；从桌面启动时向登录 shell 取 PATH 成功没有。
2. 打开该页或点 Refresh 时检查；会等命令的检查在后台做（有超时），结果只在内存里。

## 怎么算做完

- 照 DESIGN 第 10 项；只读，不改任何东西。
- 新增测试：在 PATH 上找命令（名字、路径、找不到、不可执行）、`corral --version` 的解析与出错、配置现在能否读。现有测试照样通过；clippy 无警告。
- 截图留到这一批做完一并给用户看。

## 不要做

- 不做 Saddle 的 Updates 检查和升级（DESIGN §12「这一批不做」）。
- 不写任何文件，不调用 corral 的写操作。
- 不改 Saddle 仓库。

## 完成记录

2026-10-05，paddock/main。按用户“不用写完一个就等我回复”，自查后直接合并。

- **做了什么**：`diagnostics.rs` 参照 Saddle：按启动方式在 PATH 上找命令（名字或路径、要可执行），`corral --version`（公开命令，解析 version 与 contract）、`git --version`，配置文件现在读一遍，后台运行、每个命令 5 秒超时；本地时间显示用 `libc::localtime_r`。设置窗口第四页 Diagnostics（`settings::Page` 新增，Go to Agent 也能搜到）：Commands、Agents（最近一次 `corral ls` 的时间和 agent 数或错误，侧栏记录）、Config（路径、启动时读的文件还是默认、现在能否读）、Layout（路径、启动时恢复结果、最近一次保存；保存关闭时说明）、Start（桌面或终端启动、登录 shell 的 PATH 是否取到，`main` 记录）；打开该页或点 Refresh 时检查，只在内存里；该页底部只有 Refresh。
- **验证了什么**：147 项测试通过（新增 3 项：找命令、corral 版本解析与出错、配置现在能否读；搜索测试随设置页数更新）；clippy、fmt 通过。用草稿目录的假 corral（加了 `--version`）和状态目录、临时演示代码打开该页截图：路径、版本、4 个 agent、配置、布局恢复与保存都正确显示。演示代码已删、未提交。
- **拿主意的地方**：Diagnostics 放在设置窗口而不是单独窗口（同 Saddle）；多显示 git、shell 和启动方式（DESIGN 第 10 项）。
- **没做的事**：Saddle 的 Updates 检查（这一批不做）。
