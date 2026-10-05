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
