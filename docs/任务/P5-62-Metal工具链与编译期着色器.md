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

## 完成记录

- 做了什么：在 GPUI 源码里找到 `runtime_shaders` 的做法：`gpui-pre-apple` 的构建脚本把着色器拼成一份源码（`stitched_shaders.metal`，约 66 KB），`MetalRenderer::new_internal` 每次建渲染器时用 `new_library_with_source` 编；关掉这个特性时改为 `new_library_with_data` 读编译期做好的 `shaders.metallib`。渲染器按窗口建（`gpui-pre-macos` 的 `window.rs`），所以主窗口、Settings、About 每开一个窗口都编一次。写了一个临时小程序（Rust，临时加 `metal` 0.33 依赖，GPUI 已锁；量完连同 `Cargo.toml`／`Cargo.lock` 的改动一起撤掉，没提交）用同一个调用、同一份源码计时。
- 量到的（Apple M4 Max）：
  - 第一次编：178 ms；源码末尾加一行不同的注释、强制系统缓存不命中，再量 5 次：129～131 ms。
  - 同一份源码再编：同一进程里 0 ms；新开的进程里约 1 ms，连跑 5 次都是。Metal 的系统着色器缓存跨进程、按源码命中，paddock 天天开，缓存一直是热的。
  - 拿到 Metal 设备本身 10～30 ms，两种做法都要付。
- 结论和建议：不装 Metal 工具链，`runtime_shaders` 保留。平时每开一个窗口只多约 1 ms；只有 GPUI 升级改了着色器、或系统清掉缓存（如系统更新）后的第一次启动，多 0.13～0.18 秒，而且只付一次。装了的代价反而更实在：系统组件要用户另装、编译时多一步，换机器没装组件就编不过（现在的做法不挑机器）。着色器写错要到启动才发现这一点，paddock 不改 GPUI 着色器，碰不到。以后要分发给别人用，首次启动也只多零点几秒，同样不构成理由。DESIGN §4 的 `runtime_shaders` 那条补上了量到的代价和结论。
- 没做的事：没装工具链、没改 `runtime_shaders`（按任务约定）；没量装了以后的启动时间（要先装组件，结论已经不需要）。只改文档，`git diff --check` 过。
