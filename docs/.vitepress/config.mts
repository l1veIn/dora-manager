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
      { text: '架构', link: '/07-architecture-overview' },
      {
        text: '更多',
        items: [
          { text: '节点生态', link: '/19-builtin-nodes' },
          { text: '工程实践', link: '/23-build-and-embed' },
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
          ],
        },
        {
          text: '后端架构（Rust）',
          items: [
            { text: '整体架构', link: '/07-architecture-overview' },
            { text: '数据流转译器', link: '/08-transpiler' },
            { text: '节点管理', link: '/09-node-management' },
            { text: '运行时服务', link: '/10-runtime-service' },
            { text: '事件系统', link: '/11-event-system' },
            { text: 'HTTP API', link: '/12-http-api' },
            { text: '配置体系', link: '/13-config-system' },
          ],
        },
        {
          text: '前端架构（Svelte）',
          items: [
            { text: 'SvelteKit 结构', link: '/14-sveltekit-structure' },
            { text: '可视化图编辑器', link: '/15-graph-editor' },
            { text: '运行工作台', link: '/16-runtime-workspace' },
            { text: '响应式组件', link: '/17-reactive-widgets' },
            { text: '国际化与 UI', link: '/18-i18n-and-ui' },
          ],
        },
        {
          text: '节点生态',
          items: [
            { text: '内置节点', link: '/19-builtin-nodes' },
            { text: 'Port Schema', link: '/20-port-schema' },
            { text: '交互系统', link: '/21-interaction-system' },
            { text: '自定义节点开发', link: '/22-custom-node-guide' },
          ],
        },
        {
          text: '工程实践',
          items: [
            { text: '前后端联编', link: '/23-build-and-embed' },
            { text: 'CI/CD', link: '/24-ci-cd' },
            { text: '测试策略', link: '/25-testing-strategy' },
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
          ],
        },
        {
          text: 'Backend Architecture (Rust)',
          items: [
            { text: 'Architecture Overview', link: '/en/07-architecture-overview' },
            { text: 'Transpiler', link: '/en/08-transpiler' },
            { text: 'Node Management', link: '/en/09-node-management' },
            { text: 'Runtime Service', link: '/en/10-runtime-service' },
            { text: 'Event System', link: '/en/11-event-system' },
            { text: 'HTTP API', link: '/en/12-http-api' },
            { text: 'Configuration', link: '/en/13-config-system' },
          ],
        },
        {
          text: 'Frontend Architecture (Svelte)',
          items: [
            { text: 'SvelteKit Structure', link: '/en/14-sveltekit-structure' },
            { text: 'Graph Editor', link: '/en/15-graph-editor' },
            { text: 'Runtime Workspace', link: '/en/16-runtime-workspace' },
            { text: 'Reactive Widgets', link: '/en/17-reactive-widgets' },
            { text: 'i18n and UI', link: '/en/18-i18n-and-ui' },
          ],
        },
        {
          text: 'Node Ecosystem',
          items: [
            { text: 'Built-in Nodes', link: '/en/19-builtin-nodes' },
            { text: 'Port Schema', link: '/en/20-port-schema' },
            { text: 'Interaction System', link: '/en/21-interaction-system' },
            { text: 'Custom Node Guide', link: '/en/22-custom-node-guide' },
          ],
        },
        {
          text: 'Engineering',
          items: [
            { text: 'Build and Embed', link: '/en/23-build-and-embed' },
            { text: 'CI/CD', link: '/en/24-ci-cd' },
            { text: 'Testing Strategy', link: '/en/25-testing-strategy' },
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
