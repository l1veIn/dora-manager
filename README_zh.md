# dm — Dora Manager

一个基于 Rust 构建的强大命令行工具、HTTP 接口以及可视化控制台，专为管理和编排 [dora-rs](https://github.com/dora-rs/dora) 环境而生。`dm` 远不止是一个简单的版本管理器，它内置了零网络开销（Zero-Networking）的数据流转译器、响应式的 UI 组件装配台，并提供完整的运行时编排能力。

---

## 🎨 交互式可视化编排引擎

Dora Manager 的核心亮点之一是其基于 SvelteFlow 打造的高性能可视化交互面板。你可以在浏览器中直接拼接、修改并可视化你的 Dora 数据流。

<p align="center">
  <img src="assets/editor_polish_demo.webp" alt="Graph Editor Demo" style="max-width: 100%; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.1)"/>
</p>

- **顺滑的右键上下文菜单**：在画布上轻松一键复制节点、删除连线或快速洞察属性。
- **全局悬浮配置监视器（Inspector）**：一个可拖拽、可缩放的独立视窗，根据每个节点的内建能力，动态呈现其特有的参数配置表单。
- **与底层的毫秒级实时同频**：你在画布上连的一根线、改的一项配置，都会立刻反应到底层的 YAML 模型上，绝不脱节。

### 界面概览

<table align="center">
  <tr>
    <td align="center"><b>数据流向全局视野</b></td>
    <td align="center"><b>深度 Schema 配置监听</b></td>
  </tr>
  <tr>
    <td align="center"><img src="assets/editor_screenshot.png" width="480"></td>
    <td align="center"><img src="assets/inspector_screenshot.png" width="480"></td>
  </tr>
</table>

<br/>

## 🚀 核心特性

- **极致的可视化编排**：由 Svelte/Tailwind 构建，支持流式网格布局、Tab 懒加载和数据追踪的酷炫 Web 面板。
- **活响应式挂载组件（Reactive Widgets）**：通过极为丰富的自定义组件库（滑动条、多选框、开关、路径选择器 `PathPicker`，甚至内嵌 `plyr` 视频播放器和交互式 `JSON` 树）无限赋能底层的 dora-rs 节点。
- **开箱即用的生态套件**：自带免配置的通用与 AI 算力节点：
  - `dm-downloader`：支持 SHA/MD5 校验、自动解压及前端联动的 HTTP 下载器。
  - `dm-queue`：具有元数据穿透和空闲刷写机制的高性能数据缓存队列。
  - `dora-qwen` / `dora-vad` / `dora-kokoro-tts` 等：一键在桌面端拉起多模态 AI 交互流程。
- **系统级健康侦测**：自带 `doctor` 环境探针和直观的 CPU/内存占用徽标跟踪体系。
- **“零网络代理”编译架构**：透明地将在面板里画出的连线，瞬间动态编译成完全没有 Network 损耗的单机原生数据流。

## 🏗️ 架构概览

```text
dm-core   (库)    → 承载业务核心逻辑、拓扑图翻译、Zero-Networking 状态机以及 Node 部署器
dm-cli    (终端)  → CLI 命令行工具 (带高亮、进度条与环境隔离引擎)
dm-server (接口)  → 基于 Axum 框架的 HTTP API & WebSocket 同步服务 (绑定端口 3210)
web       (前端)  → 极速响应的可视化 Web 面板，动态组装覆盖件 (Widget Overrides)
```

## 🧠 核心设计理念

从底层来讲，`dm` 是建立在 Apache Arrow 提供的高速、无拷贝跨语言共享内存总线之上的。通过三大核心抽象，它把复杂的架构编织得极其简单：

### 1. 节点 Nodes (乐高积木)
节点是语言无关的独立执行体（可以是 Rust、Python 或者是 C++ 编写）。每个节点的行为边界是由其底内的 **`dm.json` 契约文件** 严格敲定的。
* `dm.json` 规定了这个节点接收什么流数据、需要什么运行时依赖（比如 Python venv）、允许用户修改哪些配置（Schema），以及应该在前端渲染什么样的专属控件（Svelte Widgets）。
* *举个例子：* `dora-qwen` 只是一个简明要求接收 "text" 并流式回吐推理结果的纯逻辑节点。

### 2. 数据流 Dataflows (大脑编排)
Dataflows 即后缀为 `.yml` 的拓扑蓝图，决定了节点之间是如何拼接成有向无环图 (DAG) 的。
* 借由 `dora-manager`，你可以用鼠标无脑连线。在水面下，`dm` 直接把你的连线翻译成了高吞吐的共享内存管道，不同的编程语言瞬间无缝在单机上顺畅互调，且完全没有通常微服务里 REST/gRPC 那样恶心的损耗。

### 3. 生命周期 Runs (运行实例)
当你按下“启动”那一刻，数据流的蓝图就被实例化成了一场 **Run**。
* Run 是你在面板上真正能监控的“活体系”。它不仅实时记录各节点 CPU/RAM 水位、打穿标准输出日志，更能让你通过面板中那些聪明的交互组件（Smart Widgets）做局部热修改，这让机器人调试乃至桌面级 AI 开发真正具备了互动性。

---

## ⚡ 极速起步 (Quick Start)

### 1. 编译系统套件

因为 `dm-server` 是用 `rust_embed` 直接把整个 SvelteKit 前端硬核打包成单个后端的二进制包发布的，所以咱们得先编译前端的静态资源。

```bash
# 编译 SvelteKit 视觉控制台
cd web
npm install
npm run build
cd ..

# 编译 Rust 后端 (包含 dm-cli 和 dm-server)
cargo build --release
```

### 2. 进入可视化面板

万事俱备，现在一键启动你的魔法引擎：

```bash
./target/release/dm-server
```

**接着，在你的浏览器中直接访问：[http://127.0.0.1:3210](http://127.0.0.1:3210) 即可登入 Visual Editor！**

> 💡 **开发者提示**：如果你是在做二次开发，可以直接运行根目录的 `./dev.sh`，这会同时启动带有热更新（HMR）的前后端双发服务器。

### 3. 控制环境 (命令行工具 CLI)

你可以继续使用轻便强大的 CLI 工具在后台默默调度一切：

```bash
# 基础环境与生命周期检测
./target/release/dm install
./target/release/dm doctor
./target/release/dm use 0.4.1

# 流线调度 (在后台强起/摧毁节点进程)
./target/release/dm up
./target/release/dm start dataflow.yml
./target/release/dm down
```

## 🔌 HTTP API 清单

底层的 Axum RESTful 服务器默认坚守在 `3210` 端口。

```bash
curl http://127.0.0.1:3210/api/doctor
curl http://127.0.0.1:3210/api/versions
curl http://127.0.0.1:3210/api/status
curl -X POST http://127.0.0.1:3210/api/install -H 'Content-Type: application/json' -d '{"version":"0.4.1"}'
curl -X POST http://127.0.0.1:3210/api/up
curl -X POST http://127.0.0.1:3210/api/down
```

## 📦 内核安装与分发策略

1. **直接白嫖编译包**：从 GitHub Releases 列表下载（光速起飞）。
2. **源码怒编**：在没有现成包的架构上直接 `cargo build --release`。
3. 类似 `cargo-binstall` 的插件动态下行策略 (`dm-node-install`)，保证各终端在节点更新上毫无痛楚。

## 📄 授权协议 (License)

Apache-2.0
