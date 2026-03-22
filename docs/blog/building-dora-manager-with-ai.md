# 我用 AI 从零构建了一个 dora-rs 可视化编排引擎

> 一个 VibeCoding 项目的真实开发记录：从模糊的桌面 AI 想法，到可视化数据流管理系统。

## 起因：一个反复出现的模式

过去一两年里，我陆续做过几个桌面端的小工具——语音输入法、OCR 屏幕识别、实时翻译、变声器。每次做完一个，我都会发现同样的问题：这些工具本质上都是**数据流**。麦克风采集音频 → VAD 切分 → Whisper 转文字 → 发送到输入框——这和 OCR 截屏 → 识别 → 翻译 → 显示结果的结构几乎一模一样。

但每次我都在重新造轮子：进程管理、数据传递、配置系统、前端控制面板……于是我开始想：有没有一种通用的策略，能让我把这些独立的能力模块像乐高一样拼在一起？

## 发现 dora-rs

在调研过程中，我找到了 [dora-rs](https://github.com/dora-rs/dora)——一个基于 Apache Arrow 的数据流运行时框架，主要面向机器人领域。虽然我的目标是桌面应用而非机器人，但它的核心能力恰好是我需要的：

- **多语言支持**：节点可以用 Rust、Python、C++ 编写，通过共享内存零拷贝通信
- **声明式拓扑**：用 YAML 文件描述节点之间的连接关系
- **进程编排**：自动管理节点的启停和数据路由

但 dora-rs 的定位偏底层，它本身不提供包管理、可视化配置、运行时监控这些上层能力。于是我决定在它之上构建一个管理层。

## 为什么是 Apache Arrow？

dora-rs 选择 Apache Arrow 作为节点间的数据交换格式，这个选择值得单独说一下。Arrow 是一种列式内存格式标准，它的核心价值在于：**定义了一种跨语言、跨进程的内存布局约定**。当一个 Python 节点把一张图片写入共享内存，Rust 节点可以直接读取同一块内存，无需序列化/反序列化——因为双方对"这块内存里的字节怎么排列"有完全一致的理解。

对于 dm 这种需要让不同语言节点高频交换数据的场景（比如 Python 的 Whisper 模型和 Rust 的音频处理节点之间传递音频帧），零拷贝意味着几乎没有通信开销。这也是我选择 dora-rs 而不是自己用 gRPC 或 ZeroMQ 搭通信层的原因——Arrow + 共享内存这套组合已经把最难的部分解决了。

## 第一步：做 dora-rs 的 "pip + ffmpeg"

项目最早的一份设计文档标题是 **"The `pip` & `ffmpeg` of dora-rs"**。当时的想法很明确：

- **像 pip 一样管节点**：`dm add dora-qwen` 自动下载节点、创建 Python 虚拟环境、安装依赖
- **像 ffmpeg 一样组管道**：`dm pipe "webcam ! object-detection ! display"` 自动推导拓扑关系并生成 YAML

一个与 dora-rs 原生方式的重要区别是节点隔离策略：dora-rs 通常在 dataflow 级别管理一个共享的 Python 虚拟环境，而 dm 为**每个节点创建独立的 venv**，并通过 `dm.json` 暴露执行入口。这意味着 `dora-qwen` 和 `dora-vad` 可以各自依赖不同版本的库，互不干扰。

这个阶段的产出是 `dm-core` 和 `dm-cli`——一个 Rust workspace，包含节点注册表解析、依赖安装沙箱，以及 CLI 命令行工具。

## dm.json 契约的浮现

随着节点数量增加，一个问题反复出现：**dm 怎么知道一个节点需要什么？** 每个节点有不同的配置参数、不同的依赖、不同的端口——dora-rs 本身不关心这些，它只负责按照 YAML 启动进程和路由数据。

`dm.json` 不是我坐下来"设计"出来的。它是在不断给节点写安装脚本、配置表单、类型校验的过程中**自然浮现**的——我发现每次都在回答同样的问题：这个节点的入口是什么？它接受哪些输入？需要哪些环境变量？前端该渲染什么控件？最终在 AI 的帮助下，我把这些散落在各处的信息归拢成了一个统一的 JSON 文件：

```json
{
  "id": "dora-qwen",
  "executable": "dora_qwen",
  "ports": [
    { "id": "text_input", "direction": "input", "schema": { "type": "string" } },
    { "id": "text_output", "direction": "output" }
  ],
  "config_schema": {
    "model_id": {
      "type": "string",
      "default": "Qwen/Qwen2.5-7B-Instruct",
      "env": "MODEL_ID",
      "x-widget": { "type": "input" }
    },
    "temperature": {
      "type": "number",
      "default": 0.7,
      "env": "TEMPERATURE",
      "x-widget": { "type": "slider", "min": 0, "max": 2, "step": 0.1 }
    }
  },
  "dependencies": { "python": { "venv": true } }
}
```

回头来看，`dm.json` 成了整个系统的核心契约。转译器依赖它做路径解析和配置合并，前端依赖它渲染控件，安装器依赖它创建沙箱环境。但它的诞生过程没有什么顶层设计的优雅——就是把重复的工作抽象了一下。

## 转译器：从 "用户意图" 到 "dora 原生 YAML"

用户写的 YAML 使用 `node: dora-qwen` 这种人类友好的引用方式，但 dora-rs 需要的是绝对路径。转译器是一个多 pass 的编译管道：

1. **Parse**：解析 YAML，将节点分类为 Managed（有 `node:` 引用的）、Panel（内置保留节点）、External（直接用 `path:` 的）
2. **Resolve Paths**：查找 `~/.dm/nodes/<id>/dm.json`，获取 `executable` 字段，拼接为绝对路径
3. **Merge Config**：四层配置合并——行内 config > 数据流级 config > 节点级 config > schema 默认值——注入为环境变量
4. **Validate Ports**：校验连线两端的端口 schema 兼容性
5. **Inject Panel**：将 `node: dm-panel` 替换为 `path: /dm路径` + 运行时参数
6. **Emit**：输出标准 dora-rs YAML

这个转译层是整个系统的核心——它让用户在更高的抽象层工作，而不需要关心 dora-rs 的实现细节。

## 从 rerun.io 到 dm 专有节点

有了节点管理和转译器之后，下一个问题是**可视化和交互**。dora-rs 社区有一个基于 [rerun.io](https://rerun.io) 的可视化节点 `dora-rerun`，能把数据流中的图像、点云等数据渲染到 Rerun 桌面应用里。但它有明显的局限：需要额外安装 Rerun 桌面应用、只能单向查看、与 dm 的 Web 界面完全割裂。

这让我产生了一个更大的想法：既然 dm 已经有了 Web 面板，为什么不把可视化和控制直接做进去？而且我一直有做 Tauri 桌面版的打算——如果有了桌面窗口，还可以内置字幕叠加、弹幕、浮窗提示等更贴合桌面场景的专有表现层节点。

于是我规划了一组"dm 专有节点"：dm-dashboard（替代 dora-rerun，实时图表/图像显示）、dm-input（从 Web UI 向数据流注入控制信号）、dm-recorder（数据录制和回放）。后来发现这三个节点的输入/输出声明高度重叠，最终合并成了一个统一的 **dm-panel**。

## Panel 架构的演化

合并后的 dm-panel 最初有一个激进的约束：**节点本身完全不做任何网络通信**。所有数据通过 SQLite 数据库（`index.db`）中转——节点写入 assets 表，dm-server 轮询读取后推送给浏览器；反方向，浏览器操作写入 commands 表，节点轮询执行。

这个设计的好处是代码极其简单。但随着需求深入——特别是实时控制场景——纯轮询的延迟变得不可接受，最终 Web 端的交互控制改为 WebSocket 实现。

当前 Panel 的数据持久化仍有一个已知问题：Arrow 数据在存储前会被转换为 Panel 预设的几种格式（JPEG、JSON 文本等），丢失了原始的 Arrow 类型信息。更合理的做法是直接持久化为 `.parquet` 或 `.arrow` 文件，保留完整的列式结构。后来引入的 port schema 机制（在 `dm.json` 的端口定义中附加 `schema` 字段）为这个方向提供了基础——schema 描述了数据的结构和类型元信息，持久化层可以据此选择最优的存储格式。这是下一步需要重构的部分。

## 可视化编辑器

Web 面板从一开始就有数据流的列表、配置、运行历史等功能，但一直缺一个关键拼图：**直接在浏览器里可视化编辑数据流拓扑**。

技术选型上，我选择了 SvelteFlow（Svelte 版的 React Flow）。第一个版本（P1）是只读的——将 YAML 解析为节点和边，用 dagre 自动布局，能看但不能改。之后逐步加入了：

- 右键上下文菜单（复制节点、删除连线）
- 连线创建（从输出端口拖拽到输入端口）
- 浮动 Inspector 面板（根据 `dm.json` 的 config_schema 动态渲染配置表单）
- 画布变更实时回写 YAML

编辑器目前还比较基础——没有撤销/重做、没有自动布局切换、没有多选批量操作。但它已经能用了，而且对于快速理解一个数据流的结构非常有帮助。

## AI 工具的使用边界

这个项目主要使用 Antigravity（Google DeepMind）和 Codex（OpenAI）完成。2026 年，手动逐行编写这种规模的全栈项目已经不太现实，AI 工具在模式化代码（Axum handler、Svelte 组件、数据结构定义）和 debug（编译错误、类型不匹配）上的效率显而易见。

但 AI 处理不了的部分同样清晰：架构层面的权衡（转译器的 pass 怎么划分、Panel 用轮询还是 WebSocket）、产品层面的判断（三个节点该不该合并）、以及跨 Rust/Svelte/Python/YAML 四个技术栈的接口一致性——这些仍然需要人来把控。

## 当前状态与不足

- ✅ CLI 节点管理 + 一键 dataflow 运行可用
- ✅ Web 面板功能基本完整（数据流管理、运行追踪、Panel 交互控件）
- ✅ 可视化编辑器基本可用
- ❌ 测试覆盖率低，没有 CI/CD
- ❌ 图编辑器缺少撤销/重做、自动布局等高级功能
- ❌ Panel 数据持久化应改为原生 Arrow 格式
- ❌ 转译器不做环检测/拓扑排序
- ❌ dm.json 规范、节点开发指南等文档不完善
- ❌ 只在 macOS 和 Linux 上测试过

## 展望

Panel 目前已经支持加载自定义 HTML 页面作为控制面板。这个能力自然延伸出了一个更大的画面：**一个固定的 dataflow + 一套专属的前端界面，可以由 dm 打包发布为独立的桌面应用**。

想象一下：一个实时字幕工具，内核是 `dora-vad → dora-distil-whisper → subtitle-overlay` 这条数据流管道，但用户看到的只是一个带有简洁控制面板的桌面窗口。数据在底层通过 Arrow 共享内存高速流转，用户完全不需要知道什么是 dataflow、什么是 YAML——他们只需要点击"开始"。

下一步的关键是基于 Tauri 的桌面版本。Tauri 不仅提供跨平台的原生窗口，还可以：

- **内置表现层节点**：字幕叠加、弹幕、系统通知、浮窗提示——这些只有桌面应用才能提供的交互方式，可以作为 dm 专有节点直接参与数据流
- **自带 Python 运行时**：将 Python 环境打包进应用分发包，用户无需自行安装任何依赖，真正做到开箱即用

这也是我最初想做的事：不是造一个框架，而是用数据流的方式，更高效地构建桌面端的 AI 应用。

项目地址：[github.com/l1veIn/dora-manager](https://github.com/l1veIn/dora-manager)
