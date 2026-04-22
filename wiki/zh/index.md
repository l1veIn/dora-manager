---
title: Dora Manager 文档
---

# Dora Manager

Dora Manager（简称 `dm`）是一个基于 Rust 构建的 **数据流编排与管理平台**，为 [dora-rs](https://github.com/dora-rs/dora) 提供 CLI、HTTP API 和可视化 Web 面板三层管理能力。

## 快速导航

### 入门指南
- [项目概览](01-project-overview.md) — Dora Manager 是什么与为什么
- [快速开始](02-quickstart.md) — 安装、启动与运行第一个数据流
- [开发环境](03-dev-environment.md) — 从源码构建与热更新工作流

### 核心概念
- [节点（Node）](04-node-concept.md) — dm.json 契约与可执行单元
- [数据流（Dataflow）](05-dataflow-concept.md) — YAML 拓扑定义与节点连接
- [运行实例（Run）](06-run-lifecycle.md) — 生命周期状态机与指标追踪
- [内置节点](07-builtin-nodes.md) — 从媒体采集到 AI 推理
- [Port Schema](08-port-schema.md) — 端口类型校验
- [自定义节点开发](09-custom-node-guide.md) — dm.json 完整字段参考

### 深入后端架构（Rust）
- [整体架构](10-architecture-overview.md) — dm-core / dm-cli / dm-server 职责划分
- [数据流转译器](11-transpiler.md) — 多 Pass 管线与四层配置合并
- [节点管理](12-node-management.md) — 安装、导入、路径解析与沙箱隔离
- [运行时服务](13-runtime-service.md) — 启动编排、状态刷新与 CPU/内存指标采集
- [事件系统](14-event-system.md) — 可观测性模型与 XES 兼容事件存储
- [HTTP API](15-http-api.md) — REST 路由、WebSocket 实时通道与 Swagger 文档
- [配置体系](16-config-system.md) — DM_HOME 目录结构与 config.toml

### 深入前端架构（Svelte）
- [SvelteKit 项目结构](17-sveltekit-structure.md) — 路由设计、API 通信层与状态管理
- [可视化图编辑器](18-graph-editor.md) — SvelteFlow 画布、右键菜单与 YAML 双向同步
- [运行工作台](19-runtime-workspace.md) — 网格布局、面板系统与实时日志查看
- [响应式控件](20-reactive-widgets.md) — 控件注册表、动态渲染与 WebSocket 参数注入
- [国际化与 UI](21-i18n-and-ui.md) — i18n 与 UI 组件库

### 交互系统与 Bridge 机制
- [交互系统架构](22-interaction-system.md) — dm-input / dm-message / Bridge 节点注入原理
- [Capability Binding](23-capability-binding.md) — 节点能力声明与运行时角色绑定
- [媒体流架构](24-media-streaming.md) — MJPEG 采集、MediaMTX 集成与 WebRTC/HLS 分发

### 工程实践
- [前后端联编与发布](25-build-and-embed.md) — rust-embed 静态嵌入与 CI/CD 流水线
- [测试策略](26-testing-strategy.md) — 单元测试、数据流集成测试与系统测试 CheckList
- [项目宪法与设计原则](27-project-constitution.md) — 产品愿景、决策优先级与 Agent 运作规则

## 相关链接

- [GitHub 仓库](https://github.com/l1veIn/dora-manager)
- [dora-rs 官方仓库](https://github.com/dora-rs/dora)
