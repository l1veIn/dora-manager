Dora Manager 的前端是一个基于 **SvelteKit + Svelte 5** 的单页应用（SPA），通过轻量级的 HTTP 客户端和 WebSocket 与 Rust 后端（`dm-server`）通信。本文档聚焦于前端的工程骨架、通信机制、状态管理范式和路由组织方式——即支撑所有页面功能的"基座设施"。

Sources: [svelte.config.js](https://github.com/l1veIn/dora-manager/blob/master/web/svelte.config.js#L1-L15), [vite.config.ts](https://github.com/l1veIn/dora-manager/blob/master/web/vite.config.ts#L1-L16)

## 工程骨架：SPA 模式的 SvelteKit 配置

项目采用 `@sveltejs/adapter-static` 并设置 `fallback: 'index.html'`，意味着编译产物是一组纯静态文件，由后端通过 `rust_embed` 嵌入并提供服务。渲染策略上显式禁用了 SSR 和预渲染：

```typescript
export const prerender = false;  // 不预渲染任何页面
export const ssr = false;        // 完全客户端渲染
```

这一选择决定了整个前端的运行模型：**所有页面逻辑都在浏览器端执行**，不存在服务端数据预取（load functions），API 数据完全依赖客户端 `fetch` 调用。

Sources: [+layout.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+layout.ts#L1-L3), [svelte.config.js](https://github.com/l1veIn/dora-manager/blob/master/web/svelte.config.js#L1-L13)

### 核心技术栈

| 层次 | 技术选型 | 用途 |
|------|---------|------|
| 框架 | SvelteKit 2 + Svelte 5 | 路由、编译优化、Runes 响应式 |
| 构建工具 | Vite 7 | 开发服务器、HMR、生产构建 |
| UI 组件库 | shadcn-svelte (bits-ui) | 无样式原语组件（Dialog、Select、Sheet 等） |
| 样式 | Tailwind CSS v4 + tailwind-merge | 原子化 CSS、dark mode |
| 图编辑器 | @xyflow/svelte | 数据流可视化画布 |
| 网格布局 | GridStack 12 | 运行工作台的面板拖拽/缩放 |
| 代码编辑器 | svelte-codemirror-editor | YAML/JSON 编辑 |
| 国际化 | svelte-i18n | 中英双语 |
| 图标 | lucide-svelte | 统一图标集 |

Sources: [package.json](https://github.com/l1veIn/dora-manager/blob/master/web/package.json#L1-L66), [components.json](https://github.com/l1veIn/dora-manager/blob/master/web/components.json#L1-L16)

## 目录结构全景

```
web/src/
├── app.css / app.d.ts / app.html    # SvelteKit 应用入口
├── lib/                              # 共享库代码
│   ├── api.ts                        # ← HTTP 通信核心（四个函数）
│   ├── i18n.ts                       # 国际化初始化
│   ├── utils.ts                      # cn() 样式合并工具
│   ├── index.ts                      # $lib 别名入口
│   ├── stores/
│   │   └── status.svelte.ts          # 全局状态：runtime / doctor / nodes
│   ├── locales/                      # 翻译文件
│   ├── hooks/                        # 客户端 hooks
│   ├── components/
│   │   ├── ui/                       # shadcn-svelte 原子组件（30+ 目录）
│   │   ├── layout/                   # AppHeader / AppSidebar / AppFooter
│   │   ├── runs/                     # RunStatusBadge / RecentRunCard
│   │   ├── dataflows/                # DataflowRunActions
│   │   └── workspace/               # GridStack 工作台系统
│   │       ├── Workspace.svelte      # 网格容器 + GridStack 集成
│   │       ├── types.ts              # WorkspaceGridItem 类型定义
│   │       ├── widgets/              # RootWidgetWrapper（面板外壳）
│   │       └── panels/              # 面板注册表 + 五种面板实现
│   │           ├── registry.ts       # 面板注册中心
│   │           ├── types.ts          # PanelContext / PanelDefinition 类型
│   │           ├── message/          # 消息面板 + 游标分页状态机
│   │           ├── input/            # 输入控件面板
│   │           ├── chart/            # 图表面板
│   │           ├── video/            # 视频/HLS 面板
│   │           └── terminal/         # 终端日志面板
│   └── assets/
├── routes/                           # SvelteKit 文件系统路由
│   ├── +layout.svelte                # 全局布局（Sidebar + Header + ModeWatcher）
│   ├── +layout.ts                    # 禁用 SSR/预渲染
│   ├── +page.svelte                  # Dashboard 首页
│   ├── dataflows/
│   │   ├── +page.svelte              # 数据流列表
│   │   └── [id]/
│   │       ├── +page.svelte          # 数据流详情（Tabs: Graph/YAML/Meta/History）
│   │       ├── editor/+page.svelte   # 全屏图编辑器
│   │       └── components/           # GraphEditorTab / YamlEditorTab / graph/
│   ├── runs/
│   │   ├── +page.svelte              # 运行列表（分页、搜索、过滤）
│   │   └── [id]/
│   │       ├── +page.svelte          # 运行详情（Workspace + 侧栏）
│   │       ├── InteractionPane.svelte # 交互面板（Display + Input 节点桥接）
│   │       ├── RunLogViewer.svelte   # 终端日志（tail 模式轮询）
│   │       └── graph/                # RuntimeGraphView（WebSocket 驱动）
│   ├── nodes/
│   │   ├── +page.svelte              # 节点列表
│   │   └── [id]/+page.svelte         # 节点详情
│   ├── events/+page.svelte           # 事件日志
│   └── settings/+page.svelte         # 设置（版本/媒体/配置）
```

Sources: [get_dir_structure](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/registry.ts#L1-L80), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/dataflows/+page.svelte#L1-L52)

## API 通信层：极简 HTTP 客户端

整个前端的 HTTP 通信集中在一个 33 行的模块中——`$lib/api.ts`。它导出四个泛型函数，统一了所有后端交互的错误处理和响应解析：

```typescript
export const API_BASE = '/api';

export async function get<T>(path: string): Promise<T> {
    const res = await fetch(`${API_BASE}${path}`);
    if (!res.ok) throw new Error(await res.text());
    return res.json();
}
```

`post` 和 `del` 遵循相同模式：设置 `Content-Type: application/json`，在非 200 响应时抛出包含服务端错误文本的异常。`getText` 是 `get` 的纯文本变体，用于获取 YAML 源码和日志等非 JSON 响应。

Sources: [api.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/api.ts#L1-L33)

### 开发代理与生产通信

开发时，Vite 配置了代理规则将 `/api` 请求转发到 `http://127.0.0.1:3210`（dm-server 的默认端口），同时启用 WebSocket 代理（`ws: true`）：

```typescript
server: {
    proxy: {
        '/api': {
            target: 'http://127.0.0.1:3210',
            changeOrigin: true,
            ws: true    // 代理 WebSocket 升级请求
        }
    }
}
```

生产环境下，静态文件由 dm-server 通过 `rust_embed` 直接嵌入并服务，`/api` 路径自然路由到同一进程的 HTTP handler——不存在跨域问题，无需 CORS 配置。

Sources: [vite.config.ts](https://github.com/l1veIn/dora-manager/blob/master/web/vite.config.ts#L8-L15)

### API 端点调用全景

以下表格汇总了前端各页面通过 `api.ts` 发出的所有 HTTP 请求：

| 模块 | HTTP 方法 | 端点路径 | 用途 |
|------|----------|---------|------|
| Dashboard | GET | `/status`, `/doctor`, `/nodes` | 运行时健康状态 |
| Dashboard | GET | `/runs/active?metrics=true`, `/runs?limit=100` | 活跃运行与历史列表 |
| Dataflows | GET | `/dataflows`, `/media/status` | 数据流列表与媒体状态 |
| Dataflows | POST | `/dataflows/{name}`, `/dataflows/{name}/delete` | 创建/删除数据流 |
| Dataflows | POST | `/runs/start` | 启动数据流运行 |
| Dataflow Detail | GET | `/dataflows/{name}` | 获取 YAML + 元数据 |
| Dataflow Editor | POST | `/dataflows/{name}`, `/dataflows/{name}/view` | 保存 YAML 和视图布局 |
| Runs List | GET | `/runs?limit=&offset=&status=&search=` | 分页+过滤查询 |
| Runs List | POST | `/runs/delete` | 批量删除运行 |
| Run Detail | GET | `/runs/{id}`, `/runs/{id}/messages/snapshots` | 运行详情 + 消息快照 |
| Run Detail | GET | `/runs/{id}/messages?tag=input&limit=5000` | 输入值初始化 |
| Run Detail | GET | `/runs/{id}/messages?tag=input&after_seq=` | 增量输入值获取 |
| Run Detail | POST | `/runs/{id}/messages`, `/runs/{id}/stop` | 发送消息 / 停止运行 |
| Run Detail | GET | `/runs/{id}/dataflow`, `/runs/{id}/transpiled`, `/runs/{id}/view` | 查看源码/转译结果 |
| Run Logs | GET | `/runs/{id}/logs/{nodeId}`, `/runs/{id}/logs/{nodeId}/tail?offset=` | 全量/增量日志 |
| Nodes | GET | `/nodes` | 已安装节点列表 |
| Nodes | POST | `/nodes/download`, `/nodes/install`, `/nodes/uninstall` | 节点生命周期管理 |
| Events | GET | `/events/count`, `/events?limit=&offset=` | 事件计数与列表 |
| Settings | GET | `/config`, `/versions`, `/doctor`, `/media/status` | 配置与环境信息 |
| Settings | POST | `/config`, `/install`, `/use`, `/uninstall`, `/media/install` | 版本与媒体管理 |

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+page.svelte#L32-L83), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/+page.svelte#L64-L85), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/dataflows/+page.svelte#L38-L56), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L175-L315)

## 实时通信：WebSocket 与轮询双轨机制

前端使用两种互补的实时数据获取策略：

### WebSocket：运行消息流

运行详情页在 `onMount` 时建立一个**运行作用域的 WebSocket 连接**：

```typescript
const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
const socket = new WebSocket(
    `${protocol}//${window.location.host}/api/runs/${runId}/messages/ws`
);
```

连接建立后，每当后端推送通知（`socket.onmessage`），前端执行两件事：
1. 调用 `fetchSnapshots()` 刷新交互节点（dm-display）的状态快照
2. 如果通知标签是 `input`，调用 `fetchNewInputValues()` 获取增量输入值

WebSocket 断开后自动在 1 秒后重连（`scheduleMessageSocketReconnect`），组件销毁时清理连接。

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L321-L392)

### 定时轮询：状态刷新

对于不需要毫秒级实时性的数据，前端使用 `setInterval` 轮询：

| 场景 | 间隔 | 触发条件 |
|------|------|---------|
| Dashboard 运行概览 | 3 秒 | 页面可见时 |
| 运行详情（运行中） | 3 秒 | `run.status === "running"` |
| 节点日志（tail 模式） | 2 秒 | `isRunActive && nodeId` |
| 运行停止后状态确认 | 1 秒 | 最多 10 次 |

Dashboard 页面额外监听 `visibilitychange` 事件，在标签页重新可见时立即刷新数据，避免展示过时信息。

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L375-L392), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+page.svelte#L85-L103), [RunLogViewer.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/RunLogViewer.svelte#L76-L88)

## 数据获取模式与去重策略

### Promise 去重（Deduplication-in-flight）

运行详情页中多处使用"飞行中 Promise 复用"模式，避免并发请求导致的数据竞态：

```typescript
let snapshotRefreshInFlight: Promise<void> | null = null;

async function fetchSnapshots() {
    if (snapshotRefreshInFlight) return snapshotRefreshInFlight;  // 复用飞行中的请求
    snapshotRefreshInFlight = (async () => {
        // ... 实际 fetch 逻辑
    })();
    return snapshotRefreshInFlight;
}
```

`fetchInputValues` 和 `fetchNewInputValues` 遵循相同模式，确保即使 WebSocket 通知和定时器同时触发 `fetchSnapshots()`，也只会发出一个 HTTP 请求。

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L218-L262)

### 游标分页：消息历史状态机

`message-state.svelte.ts` 导出的 `createMessageHistoryState` 是一个**基于序号游标的双向分页状态机**，专门处理运行消息的时间线浏览：

```
     ← 加载更旧消息    [oldestSeq ... messages ... newestSeq]    加载更新消息 →
         before_seq          ↑ 游标分页基准 ↑           after_seq
```

三个核心方法：
- **`loadInitial`**：从最新消息开始倒序加载 50 条（`desc: true`），用于初始化视图
- **`loadNew`**：使用 `after_seq: newestSeq` 获取游标之后的新消息，用于实时追加
- **`loadOld`**：使用 `before_seq: oldestSeq` + `desc: true` 获取更早的历史，支持无限向上滚动

每个方法都内置了 `fetching` / `fetchingOld` 互斥锁，防止并发请求。消息面板（`MessagePanel.svelte`）通过监听 `context.refreshToken` 的变化来触发 `loadNew`，实现 WebSocket 驱动的增量更新。

Sources: [message-state.svelte.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/message/message-state.svelte.ts#L20-L119), [MessagePanel.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/message/MessagePanel.svelte#L113-L125)

## 全局状态管理

### 全局运行时状态

`$lib/stores/status.svelte.ts` 利用 Svelte 5 的 `$state` rune 创建了一个**模块级单例状态**，封装了 `status`（运行时状态）、`doctor`（环境诊断）和 `nodes`（节点列表）三个响应式变量：

```typescript
let status = $state<any>(null);
let doctor = $state<any>(null);
let nodes = $state<any[]>([]);
let loading = $state(true);

async function refresh(showSkeleton = false) {
    [status, doctor, nodes] = await Promise.all([
        get('/status').catch(() => null),
        get('/doctor').catch(() => null),
        get('/nodes').catch(() => []),
    ]);
}

export function useStatus() {
    return {
        get status() { return status; },
        get doctor() { return doctor; },
        // ... 暴露只读访问器 + refresh 方法
    };
}
```

`useStatus()` 返回的是一个包含 getter 的对象——外部组件可以读取最新值、调用 `refresh()`，但无法直接赋值。Dashboard 和 AppHeader 都消费这个全局状态来展示运行时版本号和环境健康度。

Sources: [status.svelte.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/stores/status.svelte.ts#L1-L35), [AppHeader.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/layout/AppHeader.svelte#L1-L20)

### 页面级局部状态

除全局状态外，每个页面组件使用 Svelte 5 的 `$state` / `$derived` / `$effect` runes 管理自己的局部状态。典型模式：

```typescript
let runs = $state<any[]>([]);           // 数据
let loading = $state(true);             // 加载状态
let currentPage = $state(1);            // UI 状态

async function fetchRuns() {             // 数据获取函数
    loading = true;
    try { runs = (await get(`/runs?...`)).runs; }
    finally { loading = false; }
}

onMount(() => { fetchRuns(); });        // 挂载时加载
```

这种"数据即状态"的模式没有引入额外的状态管理库（如 Redux 或 Zustand），依赖 Svelte 5 的细粒度响应式系统自动追踪依赖并高效更新 DOM。

Sources: [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/+page.svelte#L22-L85), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/nodes/+page.svelte#L15-L90)

## 路由组织与布局策略

### 文件系统路由

SvelteKit 的文件系统路由将应用组织为 6 个顶级页面：

```
/                    → Dashboard（概览 + 频繁数据流 + 活跃运行）
/dataflows           → 数据流列表（搜索 + 创建 + 运行）
/dataflows/[id]      → 数据流详情（Graph/YAML/Meta/History 四个 Tab）
/dataflows/[id]/editor → 全屏图编辑器（SvelteFlow 画布）
/runs                → 运行列表（分页表格 + 搜索 + 过滤 + 批量删除）
/runs/[id]           → 运行工作台（Workspace + 侧栏 + 交互面板）
/nodes               → 节点列表（安装/下载/卸载操作）
/nodes/[id]          → 节点详情
/events              → 事件日志（过滤 + 导出 XES）
/settings            → 设置（版本管理 + 媒体配置）
```

Sources: [AppSidebar.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/layout/AppSidebar.svelte#L14-L21)

### 双布局模式

根布局（`+layout.svelte`）根据当前路由选择不同的外壳结构：

- **常规路由**：`Sidebar.Provider` + `AppSidebar` + `AppHeader` + 主内容区，构成经典的侧栏导航布局
- **编辑器路由**（`/dataflows/[id]/editor`）：完全隐藏侧栏和头部，提供全屏画布空间

```svelte
{#if isEditorRoute}
    <div class="h-screen w-screen overflow-hidden">
        {@render children()}
    </div>
{:else}
    <Sidebar.Provider bind:open={appSidebarOpen}>
        <AppSidebar />
        <main><!-- AppHeader + children --></main>
    </Sidebar.Provider>
{/if}
```

侧栏的展开/折叠状态持久化到 `localStorage`，每个运行详情页也有独立的侧栏状态键。

Sources: [+layout.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+layout.svelte#L1-L54)

## 面板注册表与工作台系统

运行详情页的核心是 **Workspace 工作台**——一个基于 GridStack 的动态面板系统。面板通过注册表（`registry.ts`）统一管理，每种面板类型声明其元数据：

```typescript
export const panelRegistry: Record<PanelKind, PanelDefinition> = {
    message:  { kind: "message",  sourceMode: "history",   supportedTags: "*",       ... },
    input:    { kind: "input",    sourceMode: "snapshot",   supportedTags: ["widgets"], ... },
    chart:    { kind: "chart",    sourceMode: "snapshot",   supportedTags: ["chart"],   ... },
    video:    { kind: "video",    sourceMode: "snapshot",   supportedTags: ["stream"],  ... },
    terminal: { kind: "terminal", sourceMode: "external",   supportedTags: [],          ... },
};
```

**`sourceMode`** 决定了面板如何获取数据：

| 模式 | 数据获取方式 | 代表面板 |
|------|------------|---------|
| `history` | 通过 `createMessageHistoryState` 发起 API 请求，游标分页 | Message |
| `snapshot` | 从 `context.snapshots` 中过滤，数据由父组件通过 WebSocket 驱动刷新 | Input, Chart |
| `external` | 自行管理独立的轮询/数据源 | Terminal（日志 tail） |

`PanelContext` 作为所有面板的统一接口，向下传递 `runId`、`snapshots`、`inputValues`、`nodes`、`emitMessage` 等数据和方法，面板内部无需关心数据来源。

Sources: [registry.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/registry.ts#L1-L80), [types.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/panels/types.ts#L1-L41), [Workspace.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/components/workspace/Workspace.svelte#L1-L175)

## 交互面板：前端与 dm-display/dm-input 的桥接

`InteractionPane.svelte` 是运行作用域中处理**交互节点**的核心组件。它同时管理两类数据：

- **Streams（显示流）**：从 `snapshots` 中过滤 `kind` 为 display 的条目，根据 `render` 字段（`text`/`json`/`markdown`/`image`/`audio`/`video`）选择渲染方式。对于非 inline 的 artifact 文件，通过 `/api/runs/{runId}/artifacts/{file}` 获取内容。
- **Inputs（输入绑定）**：从 `inputs` 中提取每个输入节点的 `widgets` 配置，动态渲染控件（文本框、滑块、开关、复选框、下拉选择），用户操作后调用 `emit()` 将值编码为消息发送到后端。

Sources: [InteractionPane.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/InteractionPane.svelte#L1-L156)

## 持久化策略

前端使用两种持久化机制：

| 机制 | 存储位置 | 用途 |
|------|---------|------|
| **Workspace 布局** | `localStorage: dm-workspace-layout-{name}` | 运行工作台的面板位置和大小 |
| **侧栏状态** | `localStorage: dm-app-sidebar-open` / `dm-run-sidebar-open-{name}` | 全局侧栏和运行侧栏的折叠状态 |
| **语言偏好** | `localStorage: dm-language` | i18n 语言选择 |
| **暗色模式** | 浏览器 preference + `mode-watcher` | 主题切换 |

Sources: [+layout.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/+layout.svelte#L18-L29), [+page.svelte](https://github.com/l1veIn/dora-manager/blob/master/web/src/routes/runs/[id]/+page.svelte#L58-L67), [i18n.ts](https://github.com/l1veIn/dora-manager/blob/master/web/src/lib/i18n.ts#L10-L20)

---

了解前端骨架与通信基座后，你可以继续探索以下专题：

- **[可视化图编辑器：SvelteFlow 画布与 YAML 同步](15-graph-editor)** —— 深入 `editor/+page.svelte` 的图编辑器实现，包括节点拖放、连线校验、YAML 双向转换
- **[运行工作台：网格布局、面板系统与实时交互](16-runtime-workspace)** —— GridStack 集成细节、面板生命周期和 WebSocket 消息流处理
- **[前后端联编：rust_embed 静态嵌入与发布流程](23-build-and-embed)** —— 生产构建如何将 SvelteKit 产物嵌入 Rust 二进制文件