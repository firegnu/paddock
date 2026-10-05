# 任务 P3-3：关闭运行中的 shell 先确认

2026-10-05，paddock/main 自己做。
类型：功能变更
依据：DESIGN §12 第 3 项。Saddle（`df1c727`）关闭运行中的 shell、关闭含 shell 的标签页或退出时，会列出要结束的 shell 让用户确认，Cancel 什么都不关，agent 窗格只是断开；paddock 现在直接关。用户 10-05：确认用系统提示框（DESIGN §12「窗口与对话框按 macOS 做」）；这一批不逐件等回复。

## 要做的

1. 关窗格（⌘W、Close pane）、关标签页（⇧⌘W、Close tab、标签上的 ×）时，若其中有还活着的 shell，弹系统提示框：列出这些 shell（目录），有 agent 窗格的再说明 agent 只是断开、继续运行；按钮 End Shells / Cancel。Cancel 什么都不关。没有活着的 shell 时照旧直接关。
2. 退出（⌘Q、关主窗口）时，若有活着的 shell，同样先问（在 P3-2 的设置窗口询问之后）；Cancel 不退出。
3. 回答前布局变了（比如那个窗格自己已经关了），按回答时的情况处理，不关错。

## 怎么算做完

- 照 Saddle 的规则：会结束活着的 shell 才问，Cancel 保留一切，agent 窗格只断开。
- 新增测试覆盖提示内容的生成（哪些算要结束的 shell、agent 的说明）；现有测试照样通过；clippy 无警告。
- 提示框是系统界面，实际操作留给用户体验。

## 不要做

- 不改 shell 的结束方式。
- 不改 Saddle 仓库。
