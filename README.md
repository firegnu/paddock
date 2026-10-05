# paddock

Saddle 的 GPUI 桌面前端，目前是原型阶段。

- 现在有什么：`prototypes/gpui-terminal/`，一个 GPUI 窗口里的真实交互终端，可以跑 shell 或接入 corral agent。构建和运行见该目录的 `README.md`。
- 和 Saddle 的关系：按固定提交号引用 Saddle 公开仓库的库，复用它的 PTY 会话、终端解析和输入编码；不修改 Saddle，Saddle 也不依赖 paddock。
- 还没定的：是否做完整桌面版、走哪条迁移路线，都等用户决定。见 `docs/DESIGN.md` §7。

给 agent 的规矩见 `AGENTS.md`，当前进度见 `HANDOFF.md`。
