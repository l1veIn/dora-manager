# dm-sdk v1 — Python SDK for dora-manager

## 1. Why

Dora manager 的节点和服务函数需要一种统一、简单的方式与 dm-server 和 dm-faasd 通信。

当前情况：
- 节点通过 bridge 走 dora Arrow 端口发消息，协议复杂，样板代码多
- service 函数无任何 SDK，只能自己拼 HTTP 请求
- Web 前端与后端通信的协议与节点不一致

dm-sdk 的目标是：**让任何 Python 代码（节点、service、外部脚本）都能用最简单的几行代码发消息、拉消息、调服务。**

## 2. Design Principles

1. **零配置可用** — 在 dora dataflow 内运行时自动读环境变量
2. **显式优于隐式** — 没有全局状态，每次使用前先 `Message()` 或 `Service()` 实例化
3. **不引入新依赖** — 只用 stdlib + `requests`（可选 urllib3 降级）
4. **不做 dm-server 的事** — SDK 只是 HTTP 客户端封装，不存状态、不管理连接
5. **路径统一** — 节点、service、前端、脚本都用同一套 SDK

## 3. API Design

### 3.1 Message — 消息收发

```python
class Message:
    """
    消息操作：发消息、拉消息、订阅。
    
    在 dora dataflow 节点中调用时无需任何参数（自动读环境变量）。
    在前端或外部脚本中调用时需显式传 run_id 和 server_url。
    """

    def __init__(self, run_id: str | None = None,
                 server_url: str | None = None):
        """
        Args:
            run_id: Run 实例 ID。不传时从 DM_RUN_ID 环境变量读取。
            server_url: dm-server 地址。不传时从 DM_SERVER_URL 环境变量读取
                       （默认 http://127.0.0.1:3210）。
        Raises:
            RuntimeError: run_id 既没传也没在环境变量中找到。
        """

    def send(self, tag: str, payload: dict,
             *, from_: str | None = None) -> int:
        """
        发消息。持久化到 SQLite，WebSocket 推给前端。
        
        Args:
            tag: 消息标签，如 "text"、"detect"、"stream"、"input"
            payload: 消息内容，任意 JSON 可序列化对象
            from_: 消息来源标识。不传时自动检测调用者文件名。
        Returns:
            seq: 消息序号（用于增量拉取）
        """

    def pull(self, *, tag: str | None = None,
             from_: str | None = None,
             after_seq: int | None = None,
             before_seq: int | None = None,
             limit: int = 100) -> list[dict]:
        """
        拉取消息历史（从 SQLite）。
        
        Args:
            tag: 按标签过滤，支持逗号分隔（如 "text,detect"）
            from_: 按来源过滤，支持逗号分隔
            after_seq: 只返回大于此序号的消息（增量拉取）
            before_seq: 只返回小于此序号的消息
            limit: 最大返回条数，默认 100
        Returns:
            消息列表，按 seq 升序排列，每条含 seq/from/tag/payload/timestamp
        """

    def snapshots(self) -> list[dict]:
        """
        获取当前所有消息快照（每个 node_id+tag 组合的最新状态）。
        
        Returns:
            快照列表，每条含 node_id/tag/payload/seq/updated_at
        """

    def subscribe(self) -> "MessageSubscriber":
        """
        订阅实时消息推送（WebSocket）。
        
        返回一个迭代器/上下文管理器，持续 yield 新消息。
        
        Usage:
            with msg.subscribe() as stream:
                for event in stream:
                    print(event)
        """
```

### 3.2 Service — 服务调用

```python
class Service:
    """
    服务调用：调 faasd 上的 service 函数。
    """

    def __init__(self, faasd_url: str | None = None):
        """
        Args:
            faasd_url: dm-faasd 地址。不传时从 DM_FAASD_URL 环境变量读取
                      （默认 http://127.0.0.1:5001）。
        """

    def invoke(self, service: str, method: str = "run",
               input: dict | None = None) -> any:
        """
        同步调用一个 service 函数。
        
        Args:
            service: 服务 ID，对应 ~/.dm/functions/<id>/ 目录
            method: 方法名，对应 service.json 里 methods 数组中的 name
            input: 输入参数，任意 JSON 可序列化对象
        Returns:
            invoke 返回的 output（JSON 反序列化后的 Python 对象）
        Raises:
            ServiceNotFoundError: service 不存在（404）
            MethodNotFoundError: method 不存在
            ServiceUnavailableError: worker 池满或服务异常
        """

    def list(self) -> list[dict]:
        """
        列出所有可用 service。
        Returns:
            每个 service 的元数据：id/name/version/description/methods
        """
```

### 3.3 完整使用示例

#### 节点中使用

```python
# nodes/my-detector/main.py

import dm

def main():
    msg = dm.Message()       # 自动读 DM_RUN_ID
    svc = dm.Service()       # 自动读 DM_FAASD_URL
    
    # 每检测到一个人，发消息到前端
    msg.send("detect", {"person": 1, "confidence": 0.95})
    
    # 调服务查电量
    battery = svc.invoke("device-battery", method="check")
    if battery["level"] < 20:
        msg.send("alert", {"content": f"电量低: {battery['level']}%"})


if __name__ == "__main__":
    main()
```

#### service 函数中使用

```python
# services/faas-demo/service.py

import dm

def run(input):
    msg = dm.Message()       # faasd 需要注入 DM_RUN_ID
    chat = input.get("text", "")
    
    msg.send("chat", {"content": chat, "role": "user"})
    # ... 处理 ...
    msg.send("chat", {"content": result, "role": "assistant"})
    
    return {"result": "ok"}
```

#### 外部脚本中使用

```python
# 任意 Python 脚本
import dm

msg = dm.Message(run_id="abc-123", server_url="http://127.0.0.1:3210")
msg.send("text", {"content": "来自脚本的消息"})
```

#### Web 前端 JS 中使用（后续扩展）

```javascript
// JS 版 SDK 设计类似
import { Message, Service } from '@dora-manager/sdk'

const msg = new Message({ runId: 'xxx', serverUrl: 'http://127.0.0.1:3210' })
msg.send('text', { content: 'hello' })
```

## 4. Project Structure

```
sdk/python/
├── pyproject.toml          # Python 打包配置
├── dm/
│   ├── __init__.py         # 导出 Message, Service
│   ├── _message.py         # Message 实现
│   ├── _service.py         # Service 实现
│   └── _util.py            # 环境变量读取、调用者检测等工具函数
├── tests/
│   ├── test_message.py
│   └── test_service.py
└── README.md
```

SDK 是一个独立的 Python 包，不耦合 dm-server 或 dm-faasd 的编译流程。

## 5. Environment Variables

| 变量 | 默认值 | 用途 |
|------|--------|------|
| `DM_RUN_ID` | — | 当前运行的 Run ID，由 transpiler 自动注入 |
| `DM_SERVER_URL` | `http://127.0.0.1:3210` | dm-server HTTP 地址 |
| `DM_FAASD_URL` | `http://127.0.0.1:5001` | dm-faasd HTTP 地址 |

## 6. Non-Goals (v1)

- WebSocket 订阅（`subscribe()` 定义接口但不实现，v1 只做 send/pull/snapshots）
- JS/TS SDK（定义结构但不实现）
- 连接池、重试、熔断（用 requests 默认行为）
- 消息路由、过滤、转换
- 离线缓存

## 7. Verification

```bash
# 单元测试
pytest sdk/python/tests/

# 端到端测试（需 dm-server 运行）
python -c "
import dm
msg = dm.Message(run_id='test-e2e', server_url='http://127.0.0.1:3210')
seq = msg.send('text', {'content': 'hello'})
msgs = msg.pull(after_seq=seq - 1)
assert len(msgs) == 1
assert msgs[0]['tag'] == 'text'
print('E2E OK')
"
```
