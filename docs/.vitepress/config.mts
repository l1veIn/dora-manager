import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(defineConfig({
  title: 'Dora Manager',
  description: 'Dataflow orchestration and management platform for dora-rs',

  srcDir: '../wiki/zh',

  ignoreDeadLinks: true,

  head: [
    ['link', { rel: 'icon', href: '/dora-manager/favicon.ico' }],
  ],

  locales: {
    root: {
      label: '中文',
      lang: 'zh-CN',
      link: '/',
    },
    en: {
      label: 'English',
      lang: 'en',
      link: '/en/',
    },
  },

  themeConfig: {
    logo: '/logo.svg',

    nav: [
      { text: '入门', link: '/01-project-overview' },
      { text: '架构', link: '/10-architecture-overview' },
      {
        text: '更多',
        items: [
          { text: '交互系统', link: '/22-interaction-system' },
          { text: '工程实践', link: '/25-build-and-embed' },
        ],
      },
    ],

    sidebar: {
      '/': [
        {
          text: '入门指南',
          items: [
            { text: '项目概览', link: '/01-project-overview' },
            { text: '快速开始', link: '/02-quickstart' },
            { text: '开发环境', link: '/03-dev-environment' },
          ],
        },
        {
          text: '核心概念',
          items: [
            { text: '节点（Node）', link: '/04-node-concept' },
            { text: '数据流（Dataflow）', link: '/05-dataflow-concept' },
            { text: '运行实例（Run）', link: '/06-run-lifecycle' },
            { text: '内置节点', link: '/07-builtin-nodes' },
            { text: 'Port Schema', link: '/08-port-schema' },
            { text: '自定义节点开发', link: '/09-custom-node-guide' },
          ],
        },
        {
          text: '深入后端架构（Rust）',
          items: [
            { text: '整体架构', link: '/10-architecture-overview' },
            { text: '数据流转译器', link: '/11-transpiler' },
            { text: '节点管理', link: '/12-node-management' },
            { text: '运行时服务', link: '/13-runtime-service' },
            { text: '事件系统', link: '/14-event-system' },
            { text: 'HTTP API', link: '/15-http-api' },
            { text: '配置体系', link: '/16-config-system' },
          ],
        },
        {
          text: '深入前端架构（Svelte）',
          items: [
            { text: 'SvelteKit 结构', link: '/17-sveltekit-structure' },
            { text: '可视化图编辑器', link: '/18-graph-editor' },
            { text: '运行工作台', link: '/19-runtime-workspace' },
            { text: '响应式控件', link: '/20-reactive-widgets' },
            { text: '国际化与 UI', link: '/21-i18n-and-ui' },
          ],
        },
        {
          text: '交互系统与 Bridge 机制',
          items: [
            { text: '交互系统架构', link: '/22-interaction-system' },
            { text: 'Capability Binding', link: '/23-capability-binding' },
            { text: '媒体流架构', link: '/24-media-streaming' },
          ],
        },
        {
          text: '工程实践',
          items: [
            { text: '前后端联编与发布', link: '/25-build-and-embed' },
            { text: '测试策略', link: '/26-testing-strategy' },
            { text: '项目宪法与设计原则', link: '/27-project-constitution' },
          ],
        },
      ],
      '/en/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Project Overview', link: '/en/01-project-overview' },
            { text: 'Quick Start', link: '/en/02-quickstart' },
            { text: 'Dev Environment', link: '/en/03-dev-environment' },
          ],
        },
        {
          text: 'Core Concepts',
          items: [
            { text: 'Node', link: '/en/04-node-concept' },
            { text: 'Dataflow', link: '/en/05-dataflow-concept' },
            { text: 'Run', link: '/en/06-run-lifecycle' },
            { text: 'Built-in Nodes', link: '/en/07-builtin-nodes' },
            { text: 'Port Schema', link: '/en/08-port-schema' },
            { text: 'Custom Node Guide', link: '/en/09-custom-node-guide' },
          ],
        },
        {
          text: 'Backend Architecture (Rust)',
          items: [
            { text: 'Architecture Overview', link: '/en/10-architecture-overview' },
            { text: 'Transpiler', link: '/en/11-transpiler' },
            { text: 'Node Management', link: '/en/12-node-management' },
            { text: 'Runtime Service', link: '/en/13-runtime-service' },
            { text: 'Event System', link: '/en/14-event-system' },
            { text: 'HTTP API', link: '/en/15-http-api' },
            { text: 'Configuration', link: '/en/16-config-system' },
          ],
        },
        {
          text: 'Frontend Architecture (Svelte)',
          items: [
            { text: 'SvelteKit Structure', link: '/en/17-sveltekit-structure' },
            { text: 'Graph Editor', link: '/en/18-graph-editor' },
            { text: 'Runtime Workspace', link: '/en/19-runtime-workspace' },
            { text: 'Reactive Widgets', link: '/en/20-reactive-widgets' },
            { text: 'i18n and UI', link: '/en/21-i18n-and-ui' },
          ],
        },
        {
          text: 'Interaction & Bridge',
          items: [
            { text: 'Interaction System', link: '/en/22-interaction-system' },
            { text: 'Capability Binding', link: '/en/23-capability-binding' },
            { text: 'Media Streaming', link: '/en/24-media-streaming' },
          ],
        },
        {
          text: 'Engineering',
          items: [
            { text: 'Build and Embed', link: '/en/25-build-and-embed' },
            { text: 'Testing Strategy', link: '/en/26-testing-strategy' },
            { text: 'Project Constitution', link: '/en/27-project-constitution' },
          ],
        },
      ],
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/l1veIn/dora-manager' },
    ],

    search: {
      provider: 'local',
    },

    editLink: {
      pattern: 'https://github.com/l1veIn/dora-manager/edit/master/wiki/zh/:path',
      text: '在 GitHub 上编辑此页',
    },

    footer: {
      message: 'Released under the Apache-2.0 License.',
      copyright: 'Copyright © 2026 Dora Manager Contributors',
    },
  },
}))
