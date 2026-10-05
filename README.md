# paddock

用 GPUI 重做 Saddle 界面的独立桌面应用，第一阶段已完成。

- 现在有什么：`app/`，一个 GPUI 窗口：左侧 Agents 列表，右侧真实交互终端，可以跑 shell 或点选接入 corral agent；有主题和字体配置。构建和运行见该目录的 `README.md`。
- 和 Saddle 的关系：代码完全分开。用到的 Saddle 代码（PTY 会话、终端解析、输入编码、corral 客户端、主题色值）已迁入本仓库自己维护，不依赖 Saddle 的库；与 Saddle 生态只通过公开约定（`corral` 命令、插件协议等）打交道。不修改 Saddle，Saddle 也不依赖 paddock。
- 还没定的：迁移之后各阶段的范围、插件界面怎么做等，见 `docs/DESIGN.md` §6、§7。

给 agent 的规矩见 `AGENTS.md`，当前进度见 `HANDOFF.md`。
