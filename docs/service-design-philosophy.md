# Service: dm 的第四基石

> Status: working philosophy note  
> Companion roadmap: [dm-service-v0-roadmap.md](./design/dm-service-v0-roadmap.md)

这份文档记录 Service 为什么应该作为 dm 的独立一等概念存在。它不是现有
Bridge、Message、Capability Binding 的重命名；这些机制会继续存在。Service
先与现有机制共存，并在后续真实使用中证明哪些场景可以用更低心智负担的方式替代。

## 背景：房间里的大象

dm 现有三大基石概念：

| 概念 | 描述 |
|---|---|
| Node | 功能载体，生产或消费数据 |
| Dataflow | 节点之间的连接拓扑 |
| Run | Dataflow 的一次执行实例 |

这三个概念都围绕 **Topic 模式**（数据流/Stream）展开——节点生产数据，数据沿连接流动，消费者拿到数据。

但 dm 的真实能力不止于此。回想：

- `dm-slider` 调整参数——这不是 Topic，是"你发一个值，节点响应"
- `dm-display` 展示数据——这不是 Topic，是"你显示这个"
- `dm-message` 传递消息——这不是 Topic，是"你把这个消息存下来，供查询"

这些功能有一个共同特征：**它们是一次操作（Operation），而非持续数据流（Stream）。**

它们一直在那里。只是我们一直没有给它一个名字。

它是 **Service**。

## 核心认知：通信方式决定架构

一切回到本质上——**什么该是 Topic，什么该是 Service？**

这不是一个任意的分类。它来自一个根本的区别：

```
Topic 的本质：  数据流（Stream）。
               生产者不知道消费者是谁。
               数据持续流动，丢了一个下一个补上。
               适合：摄像头帧、传感器数据、日志流。

Service 的本质：操作（Operation）。
               请求者明确指向响应者。
               一次交互，有明确的输入和输出。
               适合：查询电量、调用检测、修改配置。
```

### 两种原语

这是两种同级但不同的通信原语。它们解决的问题不同，因此底层实现也不同：

| | 数据流（Stream） | 操作（Operation） |
|---|---|---|
| 通信模式 | 订阅-发布 | 请求-响应 |
| 数据形态 | 二进制大块数据 | 结构化小消息（JSON） |
| 性能要求 | 极高吞吐、极低延迟 | 适中，毫秒级 |
| 传输方式 | 共享内存 / Arrow 零拷贝 | JSON over HTTP / WebSocket / Socket |
| 服务对象 | 节点 ↔ 节点 | 节点 ↔ 节点 / 人 ↔ 系统 |
| 有无状态 | 无状态（丢了一个下一个） | 有状态（一对一的对话） |

**这不是技术的选择，而是语义的决定。** 因为 Topic 处理的是高速持续的数据流，所以它配得上 Arrow 零拷贝。因为 Service 处理的是离散的操作，所以 JSON 足够好，HTTP/WS 够灵活。

**同一个功能可以同时存在为 Node 和 Service：**

```
YOLO 检测
  → 作为 Node：订阅摄像头 Topic，发布检测结果 Topic，高速持续
  → 作为 Service：POST 一张图片，返回检测框，按需调用
```

底层是同一份检测代码，暴露了两种通信方式。

## Service 的本质定义

Service 可以被简单理解为：

> **一个"JSON in → JSON out"的调用入口。**

它不关心底层是 Python 函数、命令行调用、HTTP 端点还是别的什么。它只承诺一件事：你传一个 JSON 进去，你收到一个 JSON 回来。

一个服务可以有多个入口（函数），每个入口都有明确的输入/输出 Schema：

```
dm.invoke('battery').get_level()
→ input:  {}
→ output: {"level": 85}

dm.invoke('battery').charge(80)
→ input:  {"target": 80}
→ output: {"started": true, "estimated_minutes": 45}

dm.invoke('yolo').detect(image)
→ input:  {"image": "<base64>"}
→ output: {"bboxes": [...]}
```

## 做减法：ROS 2 的四原语 → dm 的两原语

ROS 2 定义了四种通信方式：Topic、Service、Action、Parameter。

dm 只需要前两个。后两者被 Service 自然覆盖，不需要新的原语。

### Action 就是 Service

Action 的本质：一个长时间执行、有进度反馈、可被取消的调用。

它不需要新的通信原语——Service 的"请求 → 响应"模型足够承载。进度的获取有多种方式，都是 Service 的调用方式，不是新原语：

```
轮询进度：
  dm.invoke('navigator').start({"x": 10, "y": 20})
  → {"task_id": "nav-123"}                              // 先返回一个 task ID
  dm.invoke('navigator').progress("nav-123")
  → {"distance": 5.2, "status": "moving"}               // 调用另一个入口查进度

SSE 推送：
  dm.invoke('navigator').subscribe("nav-123")
  → SSE 流，持续推送进度事件
  → 最终推送一条完成事件

回调通知：
  dm.invoke('navigator').start({
    "x": 10, "y": 20,
    "on_progress": "<回调地址>",
    "on_finish": "<回调地址>"
  })
  → 服务端主动回调通知进度和完成
```

无论哪种方式，它们都是 Service 调用——`json in → json out`。只是有的调用回来的结果不只一条。这是实现问题，不是原语问题。

### Parameter 就是 Service

Parameter 本质上是 **get/set 两个 Service 调用**的组合：

```
dm.invoke('yolo').get_confidence()
→ {"value": 0.5}

dm.invoke('yolo').set_confidence({"value": 0.8})
→ {"ok": true}
```

一个 get，一个 set，没有更多了。

### 归并结果

```
ROS 2 的四原语：
  Topic     → dm 保留为数据流（Stream）
  Service   → dm 保留为操作（Operation）
  Action    → 不需要新原语，Service 实现
  Parameter → 不需要新原语，Service 实现

dm 的两原语：数据流（Stream）+ 操作（Operation）
```

## Service 的三种存在模式

### 第一类：随 Run 启动的服务

有状态、有生命周期，和某一次 Dataflow 执行绑定。

需在 `service.json` 中有额外标记以声明它对 Run 的依赖——Core 在启动 Dataflow 时确保该服务可用，在 Dataflow 停止时清理服务状态。

```
dm start my-dataflow.yml

→ 启动节点
→ 同时启动 dataflow 中声明依赖的服务

POST /api/runs/run-123/service/yolo/detect
```

当通过 `invoke` 或 `POST` 调用这类服务时，调用方需要处理"服务可能尚未就绪"的状态。这与 ROS 2 中等待节点 Service 就绪是一致的——是"保证可用状态"这一业务本身的自然要求。

典型例子：YOLO 检测服务（加载模型，占用 GPU）、导航服务（初始化地图）。

### 第二类：与 Run 无关的常驻服务

全局可用，独立于任何 Dataflow。不受 Dataflow 生命周期影响。

不局限于 dm-server——只要 dm-server 在线就能找到这些服务，但它们本身不一定运行在 dm-server 进程内。它们可以是一个独立的后台守护进程、一个外部 API、一个系统命令。Core 的注册表知道它们在哪。

```
POST /api/runs/{id}/service/message/send
POST /api/runs/{id}/service/config/get
```

这里的 `{id}` 只是一个隔离域——不同 Run 的同名服务通过它区分。

典型例子：Message 服务、Config 服务、Log 服务、Service Registry、dm-server 自带的公共服务。

### 第三类：无状态服务，按需调用

不需要常驻，有请求时启动进程，执行完结束。

```bash
dm invoke calculator add --input '{"x": 1, "y": 2}'
```

典型例子：计算工具、格式转换、健康检查。

## Service 在 dm 架构中的位置

```
┌────────────────────────────────────────────────────┐
│                    dm                              │
│                                                    │
│  ~/.dm/                                            │
│  ├── nodes/        ← 数据流单元（Topic / Stream）     │
│  ├── dataflows/    ← Topic 连接关系                  │
│  ├── runs/         ← Dataflow 执行实例               │
│  └── services/     ← 操作单元（Service / Operation）  │
│       ├── message/                                  │
│       ├── config/                                   │
│       ├── yolo/                                     │
│       └── battery/                                  │
│                                                    │
│  dm-core ──→ 统一管理 Node / Dataflow / Run / Service │
│     │                                               │
│     ├── Topic 通道（转发给 dora-rs，共享内存 + Arrow） │
│     └── Service 桥（JSON → 目标 → JSON）              │
│                                                    │
│  dm-server ──→ REST + WebSocket                     │
│     ├── /api/runs/{id}           ← 数据流管理        │
│     └── /api/runs/{id}/service   ← 服务调用         │
│                                                    │
│  节点内 SDK 调用：                                   │
│    dm.invoke('battery').get_level()                 │
│                                                    │
│  Web 调用：                                         │
│    POST /api/runs/{id}/service/battery/level        │
└────────────────────────────────────────────────────┘
```

## 服务质量描述（service.json）

每个 Service 用一个 `service.json` 描述，结构类比 Node 的 `dm.json`：

```json
{
  "id": "yolo",
  "version": "0.1.0",
  "description": "YOLO object detection service",
  "provides": [
    {
      "name": "detect",
      "type": "service",
      "input": {
        "image": {"type": "string", "description": "base64 encoded image"}
      },
      "output": {
        "bboxes": {"type": "array", "description": "detection results"}
      }
    },
    {
      "name": "load_model",
      "type": "action",
      "description": "Load a YOLO model (takes time, has progress)",
      "input": {
        "model_name": {"type": "string"}
      },
      "progress": {
        "percentage": {"type": "float"},
        "stage": {"type": "string"}
      },
      "output": {
        "ok": {"type": "bool"}
      }
    }
  ],
  "exec": "python service.py"
}
```

## run_id 的含义

请求路径中的 `{id}` 作为**默认上下文参数**传入服务，服务自己决定如何使用：

```python
def handle(request):
    run_id = request.context.run_id
    # 可以用来隔离不同 run 的日志
    # 可以用来取某个 run 的配置
    # 可以完全忽略
```

不需要赋予更多语义。Core 只管传递，服务自己决定。

## 总结

1. **Topic 和 Service 是两种同级但不同的通信原语**——数据流 vs 操作
2. **通信方式决定底层实现**——共享内存 + Arrow 给 Topic，JSON over HTTP/WS 给 Service
3. **ROS 2 的 Action 和 Parameter 可被 Service 自然覆盖**——dm 只需要两原语
4. **Service 是 dm 的第四基石**——与 Node、Dataflow、Run 同层级
5. **Service 与 Node 的关系是互补而非替代**——同一功能可同时以两种形态存在
