# 任务：量一下启动时编着色器花多少时间，再定要不要装 Metal 工具链

2026-10-09，paddock/main 写，主控自己做。
类型：调研（只量、只写结论，不改程序、不装东西）
依据：
- 用户 10-08 问 “Metal 工具链：要不要装，现在靠 runtime_shaders 绕过去 这是什么？”，听完主控解释后：“这个加一个任务到draft中”。
- 用户 10-09：“p5-62你自己做”。草稿里主控的建议是“先量，有数字再定装不装”，按这个做。
执行：主控

## 在哪里干活
- worktree：`/Users/firegnu/Developer/personal_projs/paddock-worktrees/p5-62-shader-timing`，分支 `p5-62-shader-timing`（已从 main 建好）。

## 现状
- GPUI 在 macOS 上用 Metal 画界面，默认在编译期用 Xcode 的 Metal Toolchain 组件编着色器；本机 Xcode 27 没装这个组件，默认方式编译会报 `missing Metal Toolchain; use: xcodebuild -downloadComponent MetalToolchain`。
- 安装属于系统安装，一直没做；从原型起打开 `gpui-pre-platform` 的 `runtime_shaders`（`app/Cargo.toml`），每次启动时编着色器（DESIGN §4、`docs/原型实测记录.md`）。
- 代价没单独量过：当时只知道从启动到窗口出现在 3 秒内。

## 要做的
1. 找到 `runtime_shaders` 下 GPUI 启动时编的那份着色器源码，用临时小程序（Rust，不进 main）在本机计时：同一份源码用 Metal 运行时编译要多久，多跑几次看第一次和之后的差别（系统有没有缓存）。
2. 把数字、怎么量的、主控的建议（装不装、装了要改什么）写进本文件的完成记录；DESIGN §4 那条补上量到的代价。
3. 不装 Metal 工具链、不改 `runtime_shaders`：装不装由用户看了数字再定，装由用户在终端跑。

## 怎么算做完
- 用户原话：“p5-62你自己做”；草稿里的“先量，有数字再定装不装”。
- 有实测数字和建议；临时程序不进 main；`app/` 下检查照常通过（只改文档时 `git diff --check`）。
