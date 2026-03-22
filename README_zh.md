# dm — Dora Manager

基于 Rust 构建的 CLI 工具、HTTP API 与可视化管理面板，用于管理 [dora-rs](https://github.com/dora-rs/dora) 运行环境。`dm` 在 dora-rs 之上提供了数据流转译器（Transpiler）、可扩展的 UI 组件系统以及完整的运行时编排能力。

---

## 🎨 可视化图编辑器

基于 SvelteFlow 构建的交互式数据流编辑器，支持在浏览器中直接构建、编辑和预览 Dora 数据流拓扑。

<p align="center">
  <img src="assets/editor_polish_demo.webp" alt="Graph Editor Demo" style="max-width: 100%; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.1)"/>
</p>

- **右键上下文菜单**：支持节点复制、连线删除、节点属性查看等常用操作。
- **浮动 Inspector 面板**：可拖拽、可调整大小的独立窗口，根据节点的 `dm.json` 定义动态渲染配置表单。
- **实时同步**：画布上的所有编辑操作（连线、属性修改等）实时反映到底层 YAML 模型。

### 界面预览

<table align="center">
  <tr>
    <td align="center"><b>数据流全局视图</b></td>
    <td align="center"><b>Inspector 配置面板</b></td>
  </tr>
  <tr>
    <td align="center"><img src="assets/editor_screenshot.png" width="480"></td>
    <td align="center"><img src="assets/inspector_screenshot.png" width="480"></td>
  </tr>
</table>

<br/>

## 🚀 核心特性

- **可视化数据流编排**：基于 Svelte/Tailwind 构建的 Web 面板，支持网格布局、Tab 懒加载与运行状态追踪。
- **可扩展的响应式组件系统（Reactive Widgets）**：提供滑动条、多选框、开关、路径选择器（`PathPicker`）、视频播放器（`plyr`）、JSON 树等丰富的交互组件，节点可通过 `dm.json` 声明所需的控件类型。
- **内置节点生态**：
  - `dm-downloader`：支持 SHA/MD5 校验、自动解压的 HTTP 文件下载节点。
  - `dm-queue`：支持元数据透传和空闲刷写的高性能缓冲队列。
  - `dora-qwen` / `dora-vad` / `dora-kokoro-tts`：可直接用于构建语音交互或多模态 AI 流程。
- **环境健康诊断**：内置 `doctor` 命令进行环境探测，提供 CPU/内存使用率指标追踪。
- **数据流转译器（Transpiler）**：将用户编写的扩展 YAML（包含 `node:` 引用和 `config:` 配置）自动翻译为 dora-rs 原生可执行的标准 YAML，过程包括节点路径解析、配置四层合并（inline > flow > node > schema default）注入为环境变量、Panel 节点注入以及端口 Schema 兼容性校验。

## 🏗️ 项目结构

```text
dm-core   (lib)   → 核心逻辑层：转译器、节点管理、运行调度
dm-cli    (bin)   → 命令行工具（彩色输出、进度条）
dm-server (bin)   → 基于 Axum 的 HTTP API 服务（默认端口 3210）
web       (Svelte)→ Web 可视化面板，支持 WebSocket 实时交互
```

## 🧠 设计理念

`dm` 将 [dora-rs](https://github.com/dora-rs/dora) 作为底层的**多语言数据流运行时**——一个基于 Apache Arrow 构建的高性能进程编排引擎，支持 Rust、Python、C++ 等语言编写的节点之间通过共享内存进行零拷贝通信。在此运行时之上，`dm` 提供了一层管理抽象，围绕三个核心实体组织工作：

### 1. 节点（Nodes）
节点是语言无关的独立可执行单元（支持 Rust、Python、C++）。每个节点的接口与行为由 **`dm.json`** 文件定义。

`dm.json` 是 dm 体系中的核心契约文件，包含以下声明：
- **`executable`**：节点的可执行入口路径
- **`ports`**：输入/输出端口定义，可附带 `schema` 用于转译时的类型兼容性校验
- **`config_schema`**：节点的配置模式，定义可配置参数及其类型、默认值和环境变量映射（`env` 字段）
- **`widgets`**：声明该节点在 Web Panel 中需要渲染的前端控件类型
- **`dependencies`**：运行时依赖声明（如 Python 虚拟环境路径）

示例：`dora-qwen` 节点接收文本输入，输出流式推理结果，其 `config_schema` 定义了模型路径、温度等可配参数。

### 2. 数据流（Dataflows）
数据流是 `.yml` 格式的拓扑定义文件，描述节点实例之间的连接关系。

用户在 YAML 中使用 `node:` 字段引用已安装节点（而非直接指定 `path:`），dm 的转译器会在启动时完成以下工作：
1. 将 `node: dora-qwen` 解析为节点实际可执行文件的绝对路径
2. 将 `config:` 中的配置值与节点级、数据流级和 schema 默认值四层合并后注入为环境变量
3. 校验连线两端端口的 schema 兼容性
4. 注入 Panel 节点的运行时参数

> **注意**：数据流目前仅对端口入度进行限制（每个输入端口最多一个来源），不强制要求拓扑为 DAG（不做环检测）。

### 3. 运行实例（Runs）
启动数据流后，系统会创建一个 **Run** 实例来追踪本次执行的完整生命周期。
- Run 记录各节点的 CPU/内存使用情况和标准输出/错误日志。
- Web Panel 通过 WebSocket 与运行中的节点实时交互，支持通过 Smart Widgets 动态调整运行参数。

---

## ⚡ 快速开始

### 1. 编译

`dm-server` 使用 `rust_embed` 将 SvelteKit 前端静态嵌入到 Rust 二进制中，因此需要先编译前端资源。

```bash
# 编译前端
cd web
npm install
npm run build
cd ..

# 编译 Rust 后端（dm-cli 和 dm-server）
cargo build --release
```

### 2. 启动服务

```bash
./target/release/dm-server
```

**在浏览器中访问 [http://127.0.0.1:3210](http://127.0.0.1:3210) 进入可视化管理面板。**

> 💡 **开发模式**：运行 `./dev.sh` 可同时启动带有热更新（HMR）的前端开发服务器和 Rust 后端。

### 3. CLI 工具

```bash
# 环境管理
./target/release/dm install
./target/release/dm doctor
./target/release/dm use 0.4.1

# 数据流执行
./target/release/dm up
./target/release/dm start dataflow.yml
./target/release/dm down
```

## 🔌 HTTP API

Axum 服务默认监听 `3210` 端口。

```bash
curl http://127.0.0.1:3210/api/doctor
curl http://127.0.0.1:3210/api/versions
curl http://127.0.0.1:3210/api/status
curl -X POST http://127.0.0.1:3210/api/install -H 'Content-Type: application/json' -d '{"version":"0.4.1"}'
curl -X POST http://127.0.0.1:3210/api/up
curl -X POST http://127.0.0.1:3210/api/down
```

## ⚠️ 当前局限与改进方向

`dm` 目前仍处于活跃开发阶段，以下是已知的不足：

- **图编辑器尚不成熟**：可视化编辑器功能基本可用，但缺少自动布局、撤销/重做、多选批量操作等高级功能。
- **缺少自动化测试覆盖**：项目整体的单元测试和集成测试覆盖率较低，缺乏 CI/CD 流程。
- **节点安装依赖网络**：`dm install` 和节点下载依赖 GitHub Releases，不支持离线安装。
- **仅支持单机部署**：当前架构不支持分布式多机集群调度。
- **无拓扑校验**：转译器不进行环检测或拓扑排序，仅做端口入度限制和 Schema 兼容性校验。
- **文档待完善**：`dm.json` 完整规范、节点开发指南、API 参考文档尚未补齐。
- **Windows 兼容性未验证**：主要在 macOS 和 Linux 上开发测试。

## 📦 安装方式

1. **预编译二进制**：从 GitHub Releases 下载对应平台的二进制文件。
2. **源码编译**：通过 `cargo build --release` 从源码构建。
3. **节点分发**：`dm-node-install` 采用类似 `cargo-binstall` 的策略进行节点包的下载与安装。

## 🤖 开发说明

本项目 [VibeCoding](https://en.wikipedia.org/wiki/Vibe_coding) 含量较高。代码的大部分内容借助 AI 编程工具完成，主要使用了 **Antigravity**（Google DeepMind）和 **Codex**（OpenAI）。人工主要负责架构决策、产品设计和质量审查。

## 📄 License

Apache-2.0
