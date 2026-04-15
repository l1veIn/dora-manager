---
title: Dora Manager 文档
---

# Dora Manager

Dora Manager（简称 `dm`）是一个基于 Rust 构建的 **数据流编排与管理平台**，为 [dora-rs](https://github.com/dora-rs/dora) 提供 CLI、HTTP API 和可视化 Web 面板三层管理能力。

## 快速导航

### 入门指南
- [项目概览](01-project-overview.md) — Dora Manager 是什么与为什么
- [快速开始](02-quickstart.md) — 构建、启动与运行第一个数据流
- [开发环境](03-dev-environment.md) — 开发环境搭建与热更新工作流

### 核心概念
- [节点（Node）](04-node-concept.md) — dm.json 契约与可执行单元
- [数据流（Dataflow）](05-dataflow-concept.md) — YAML 拓扑与节点连接
- [运行实例（Run）](06-run-lifecycle.md) — 生命周期、状态与指标追踪

### 后端架构（Rust）
- [整体架构](07-architecture-overview.md) — dm-core / dm-cli / dm-server 分层设计
- [数据流转译器](08-transpiler.md) — 多 Pass 管线与四层配置合并
- [节点管理](09-node-management.md) — 安装、导入、路径解析与沙箱隔离
- [运行时服务](10-runtime-service.md) — 启动编排、状态刷新与指标采集
- [事件系统](11-event-system.md) — 可观测性模型与 XES 兼容存储
- [HTTP API](12-http-api.md) — 路由全览与 Swagger 文档
- [配置体系](13-config-system.md) — DM_HOME 目录结构与 config.toml

### 前端架构（Svelte）
- [SvelteKit 项目结构](14-sveltekit-structure.md) — 与 API 通信层
- [可视化图编辑器](15-graph-editor.md) — SvelteFlow 画布与 YAML 同步
- [运行工作台](16-runtime-workspace.md) — 网格布局、面板系统与实时交互
- [响应式组件](17-reactive-widgets.md) — 控件注册表与动态渲染
- [国际化与 UI](18-i18n-and-ui.md) — i18n 与 UI 组件库

### 节点生态
- [内置节点](19-builtin-nodes.md) — 从媒体采集到 AI 推理
- [Port Schema](20-port-schema.md) — 基于 Arrow 类型系统的端口校验
- [交互系统](21-interaction-system.md) — dm-input / dm-display / WebSocket 消息流
- [自定义节点开发](22-custom-node-guide.md) — dm.json 完整字段参考

### 工程实践
- [前后端联编](23-build-and-embed.md) — rust_embed 静态嵌入与发布流程
- [CI/CD](24-ci-cd.md) — GitHub Actions 构建与发布配置
- [测试策略](25-testing-strategy.md) — 数据流集成测试策略与 CheckList

## 相关链接

- [GitHub 仓库](https://github.com/l1veIn/dora-manager)
- [dora-rs 官方仓库](https://github.com/dora-rs/dora)
