# DM Run Instance 设计草案

## 背景

当前 DM 对 dataflow 的运行管理依赖解析 `dora list` 的文本输出，缺少自己的 Run 状态注册表。
这导致的问题：
- Stop 无法精确定位 UUID（`dora stop` 无参数进入交互模式）
- 前端无法感知哪个 dataflow 正在运行
- Run 历史没有关联到 dataflow 来源文件
- 无法回看某次运行时使用的 dataflow 配置

## 核心概念

### 三层模型

```
Dataflow（定义层）    →  ~/.dm/dataflows/qwen-dev.yml
    │
    ▼
Run Instance（运行层） →  ~/.dm/runs/<run_uuid>/
    │
    ▼
Panel Session（数据层）→  ~/.dm/runs/<run_uuid>/panel/index.db
```

### Run 实例目录结构

```
~/.dm/runs/<run_uuid>/
├── run.json           # Run 实例元数据
├── dataflow.yml       # 启动时的 dataflow 配置副本（snapshot）
├── logs/              # 各节点日志
│   ├── node-a.log
│   └── node-b.log
└── panel/             # Panel 数据（从 ~/.dm/panel/ 迁移过来）
    └── index.db
```

### run.json 字段定义

```json
{
  "run_id": "uuid (DM 生成)",
  "dora_uuid": "dora 分配的 UUID（来自 dora start 输出）",
  "dataflow_name": "qwen-dev.yml",
  "dataflow_hash": "sha256:...",
  "status": "running | stopped | failed",
  "started_at": "2026-03-05T23:00:00+08:00",
  "stopped_at": null,
  "nodes": ["dora-microphone", "dora-vad", "dora-qwen", "panel"]
}
```

## 关键流程

### Start

1. 用户点击 Run → DM 生成 `run_id`
2. 计算 dataflow 文件的 SHA256 → `dataflow_hash`
3. 创建 `~/.dm/runs/<run_id>/` 目录
4. 复制 dataflow.yml 副本到目录中
5. 调用 `dora start` → 获取 `dora_uuid`
6. 写入 `run.json`（status: running）
7. 前端跳转到 Panel 页面

### Stop

1. 读取 `run.json` 获取 `dora_uuid`
2. 调用 `dora stop <dora_uuid>`
3. 更新 `run.json`（status: stopped, stopped_at）

### 重复运行策略

- 同一个 dataflow 尝试运行时，检查是否存在 active run
- 如果存在 → 前端弹窗提示：
  - "该 dataflow 已在运行中，停止当前运行并重新启动？"
  - "忽略，继续查看当前运行"
- 不硬性阻止，把决策权交给用户

## API 设计

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/runs` | 列出所有 Run（含 status） |
| GET | `/api/runs/:id` | 获取单个 Run 详情 |
| GET | `/api/runs/:id/dataflow` | 获取该 Run 的 dataflow 副本 |
| GET | `/api/runs/:id/logs/:node` | 获取节点日志 |
| POST | `/api/runs/start` | 启动新 Run（传入 dataflow name） |
| POST | `/api/runs/:id/stop` | 停止指定 Run |
| GET | `/api/runs/active` | 获取当前活跃的 Run（快捷查询） |

## 可参考系统

- **Kubernetes / Argo Workflows**：Pod 生命周期管理
- **Airflow / Prefect**：DAG → Run → Task 三层模型
- **Docker Compose**：up/down/ps/logs 命令模式

## 迁移计划

1. 现有 `~/.dm/run/out/` 数据迁移到新的 `~/.dm/runs/` 结构
2. 现有 `~/.dm/panel/<run_id>/` 迁移到 `~/.dm/runs/<run_id>/panel/`
3. 对应的 core/server/web 代码适配
