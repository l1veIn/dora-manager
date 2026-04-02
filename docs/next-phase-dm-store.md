# Storage Family Nodes: dm-log, dm-save, dm-recorder

> Status: **Ready for implementation**

## 统一设计原则

1. **所有存储族节点的核心逻辑一致**：接收 Arrow → 尝试序列化 → 持久化到磁盘
2. **无法序列化则拒绝**：节点 stderr 输出错误信息，跳过该事件，不崩溃
3. **路径统一约定**：所有输出路径相对于 `runs/:id/out/`
4. **[DM-IO] 日志约定**：每次成功写入打印 `[DM-IO]` 便于 RuntimeMonitor 追踪
5. **不造轮子**：能用成熟第三方库就用

## 路径约定

```
~/.dm/runs/:id/
├── run.json
├── logs/              # dora 节点原生日志
└── out/               # 存储族节点产物目录
    ├── chat.log       # dm-log 产物
    ├── chat.log.1.gz  # dm-log 轮转归档
    ├── frames/        # dm-save 产物
    │   ├── 0001.png
    │   └── 0002.png
    └── recording.wav  # dm-recorder 产物
```

Run 删除时 `out/` 整体清理。

---

## Node 1: dm-log

### 职责

Arrow → 文本序列化 → 日志文件。底层 `loguru`。

### 适用数据

文本、JSON 字符串、结构化数据。如果 Arrow 数据无法解码为文本，拒绝并报错。

### 接口

```yaml
- id: log-chat
  node: dm-log
  inputs:
    data: llm/response
  config:
    path: "chat.log"
    format: text              # text | json | csv
    rotation: "50 MB"         # loguru: "50 MB" / "1 day" / "12:00"
    retention: "7 days"       # loguru: "7 days" / "10 files"
    compression: null         # null / "zip" / "gz"
    timestamp: true           # 每条记录加时间戳前缀
```

### Config

| Field | Type | Default | Required | Description |
|-------|------|---------|----------|-------------|
| `path` | string | — | ✅ | 输出路径，相对于 `runs/:id/out/`。目录则自动生成 `{node_id}.log` |
| `format` | `text\|json\|csv` | `text` | | 序列化格式 |
| `rotation` | string/null | `null` | | loguru rotation 语法 |
| `retention` | string/null | `null` | | loguru retention 语法 |
| `compression` | string/null | `null` | | 轮转后压缩：`zip` / `gz` |
| `timestamp` | bool | `true` | | 每条记录是否加时间戳 |

### 实现

```python
from dora import Node
from loguru import logger
import json, os, sys

node = Node()
cfg = node.config
run_out = os.environ.get("DM_RUN_OUT_DIR", ".")

path = cfg["path"]
if os.path.splitext(path)[1] == "":
    path = os.path.join(path, f"{node.node_id()}.log")
log_path = os.path.join(run_out, path)
os.makedirs(os.path.dirname(log_path), exist_ok=True)

fmt = cfg.get("format", "text")
add_ts = cfg.get("timestamp", True)

sink_id = logger.add(
    log_path,
    format="{message}",
    rotation=cfg.get("rotation"),
    retention=cfg.get("retention"),
    compression=cfg.get("compression"),
)

for event in node:
    if event["type"] != "INPUT" or event["id"] != "data":
        continue

    raw = event["value"]

    # Try to serialize
    try:
        if fmt == "text":
            py = raw.as_py()
            line = py.decode("utf-8") if isinstance(py, bytes) else str(py)
        elif fmt == "json":
            line = json.dumps(raw.to_pylist(), ensure_ascii=False)
        elif fmt == "csv":
            # CSV logic: header on first write, then rows
            line = _to_csv_line(raw)
        else:
            raise ValueError(f"Unknown format: {fmt}")
    except Exception as e:
        print(f"[dm-log] Rejected: cannot serialize as {fmt}: {e}", file=sys.stderr)
        continue

    if add_ts:
        from datetime import datetime
        line = f"{datetime.now().isoformat()} | {line}"

    logger.info(line)
    print(f"[DM-IO] LOG {fmt} -> {path}")
```

### 依赖

```
loguru
```

---

## Node 2: dm-save

### 职责

Arrow → 二进制原始字节 → 独立文件。每个事件产生一个文件。标准库实现。

### 适用数据

图片帧、原始二进制 blob、任何 bytes 数据。如果 Arrow 数据无法提取为 bytes，拒绝并报错。

### 接口

```yaml
- id: save-frames
  node: dm-save
  inputs:
    data: camera/frame
  outputs: [path]
  config:
    dir: "frames/"
    naming: "{timestamp}_{seq}"
    extension: "auto"          # auto | png | jpg | bin | ...
    max_files: 1000
    max_total_size: "1 GB"
    max_age: "24h"
```

### Config

| Field | Type | Default | Required | Description |
|-------|------|---------|----------|-------------|
| `dir` | string | — | ✅ | 输出目录，相对于 `runs/:id/out/` |
| `naming` | string | `"{timestamp}_{seq}"` | | 文件名模板：`{timestamp}`, `{seq}`, `{node_id}` |
| `extension` | string | `"auto"` | | 文件扩展名。`auto` 从 Arrow metadata `dm_type` 推断 |
| `max_files` | int/null | `null` | | 最大保留文件数，超出删最旧 |
| `max_total_size` | string/null | `null` | | 最大总空间，超出删最旧 |
| `max_age` | string/null | `null` | | 最大保留时间，超出删除 |
| `overwrite_latest` | bool | `false` | | 始终覆盖同一文件（单文件滚动模式） |

### 清理优先级

`max_age` → `max_files` → `max_total_size`

### 实现

```python
from dora import Node
import os, sys, time, glob

node = Node()
cfg = node.config
run_out = os.environ.get("DM_RUN_OUT_DIR", ".")
out_dir = os.path.join(run_out, cfg["dir"])
os.makedirs(out_dir, exist_ok=True)

naming = cfg.get("naming", "{timestamp}_{seq}")
ext_cfg = cfg.get("extension", "auto")
max_files = cfg.get("max_files")
max_total = _parse_size(cfg.get("max_total_size"))
max_age = _parse_duration(cfg.get("max_age"))
overwrite = cfg.get("overwrite_latest", False)
seq = 0

MIME_TO_EXT = {
    "image/png": "png", "image/jpeg": "jpg",
    "audio/wav": "wav", "video/mp4": "mp4",
}

def cleanup():
    files = sorted(glob.glob(os.path.join(out_dir, "*")), key=os.path.getmtime)
    now = time.time()
    if max_age:
        files = [f for f in files if now - os.path.getmtime(f) <= max_age or not os.remove(f)]
    if max_files and len(files) > max_files:
        for f in files[:len(files) - max_files]:
            os.remove(f)
        files = files[len(files) - max_files:]
    if max_total:
        total = sum(os.path.getsize(f) for f in files)
        while total > max_total and files:
            total -= os.path.getsize(files[0])
            os.remove(files.pop(0))


for event in node:
    if event["type"] != "INPUT" or event["id"] != "data":
        continue

    raw = event["value"]

    # Try to extract bytes
    try:
        py = raw.as_py()
        blob = py if isinstance(py, bytes) else bytes(py)
    except Exception as e:
        print(f"[dm-save] Rejected: cannot extract bytes: {e}", file=sys.stderr)
        continue

    # Resolve extension
    ext = ext_cfg
    if ext == "auto":
        dm_type = event.get("metadata", {}).get("dm_type", "")
        ext = MIME_TO_EXT.get(dm_type, "bin")

    # Generate filename
    seq += 1
    if overwrite:
        filename = f"{naming.split('{')[0].rstrip('_')}.{ext}"
    else:
        ts = time.strftime("%Y%m%d_%H%M%S")
        filename = f"{naming.format(timestamp=ts, seq=f'{seq:04d}', node_id=node.node_id())}.{ext}"
    filepath = os.path.join(out_dir, filename)

    with open(filepath, "wb") as f:
        f.write(blob)

    cleanup()
    node.send_output("path", pa.array([filepath]))
    print(f"[DM-IO] SAVE {ext} -> {filepath} ({len(blob)} bytes)")
```

### 依赖

```
(none — stdlib only)
```

---

## Node 3: dm-recorder

### 职责

Arrow → 流式编解码 → 媒体容器文件（wav / mp4）。持续追加写入单个文件。第三方编解码库。

### 与 dm-save 的本质区别

| | dm-save | dm-recorder |
|--|---------|-------------|
| 写入模式 | 每事件一个文件 | 持续追加到单个容器文件 |
| 数据理解 | 不理解内容（raw bytes） | 理解格式（PCM→wav, frames→mp4） |
| 编解码 | 无 | 有（写 wav header, mux mp4） |
| 典型用途 | 截图、快照 | 录音、录屏 |

### 适用数据

- 音频 PCM 流 → wav 文件
- 视频帧流 → mp4 文件（需要 ffmpeg）
- 如果输入的 Arrow 数据无法识别为支持的媒体类型，拒绝并报错

### 接口

```yaml
- id: record-audio
  node: dm-recorder
  inputs:
    data: mic/audio_chunk
  outputs: [path]
  config:
    path: "recording.wav"
    media_type: "audio/pcm"    # audio/pcm | video/raw
    sample_rate: 16000         # 音频: 采样率
    channels: 1                # 音频: 声道数
    max_duration: "30m"        # 最大录制时长
    max_size: "500 MB"         # 最大文件大小
```

### Config

| Field | Type | Default | Required | Description |
|-------|------|---------|----------|-------------|
| `path` | string | — | ✅ | 输出文件路径，相对于 `runs/:id/out/` |
| `media_type` | string | — | ✅ | 输入媒体类型：`audio/pcm`, `video/raw` |
| `sample_rate` | int | `16000` | | 音频采样率 |
| `channels` | int | `1` | | 音频声道数 |
| `sample_width` | int | `2` | | 音频采样位宽（bytes） |
| `fps` | int | `30` | | 视频帧率 |
| `resolution` | string | `null` | | 视频分辨率 `"1920x1080"` |
| `max_duration` | string/null | `null` | | 最大录制时长，超出则分片（如 `"30m"`） |
| `max_size` | string/null | `null` | | 最大文件大小，超出则分片 |

### 实现（音频 wav 示例）

```python
from dora import Node
import os, sys, wave, struct, time

node = Node()
cfg = node.config
run_out = os.environ.get("DM_RUN_OUT_DIR", ".")
out_path = os.path.join(run_out, cfg["path"])
os.makedirs(os.path.dirname(out_path) or ".", exist_ok=True)

media = cfg["media_type"]
sr = cfg.get("sample_rate", 16000)
ch = cfg.get("channels", 1)
sw = cfg.get("sample_width", 2)
max_dur = _parse_duration(cfg.get("max_duration"))
max_size = _parse_size(cfg.get("max_size"))

if media == "audio/pcm":
    wf = wave.open(out_path, "wb")
    wf.setnchannels(ch)
    wf.setsampwidth(sw)
    wf.setframerate(sr)
    start_time = time.time()
    seg = 0

    for event in node:
        if event["type"] != "INPUT" or event["id"] != "data":
            continue

        raw = event["value"]
        try:
            py = raw.as_py()
            pcm = py if isinstance(py, bytes) else bytes(py)
        except Exception as e:
            print(f"[dm-recorder] Rejected: {e}", file=sys.stderr)
            continue

        wf.writeframes(pcm)

        # Check limits → split file
        elapsed = time.time() - start_time
        fsize = os.path.getsize(out_path)
        if (max_dur and elapsed >= max_dur) or (max_size and fsize >= max_size):
            wf.close()
            seg += 1
            base, ext = os.path.splitext(out_path)
            new_path = f"{base}_{seg:03d}{ext}"
            os.rename(out_path, new_path)
            node.send_output("path", pa.array([new_path]))
            print(f"[DM-IO] RECORD wav segment -> {new_path} ({fsize} bytes)")
            # Start new segment
            wf = wave.open(out_path, "wb")
            wf.setnchannels(ch)
            wf.setsampwidth(sw)
            wf.setframerate(sr)
            start_time = time.time()
        else:
            print(f"[DM-IO] RECORD wav += {len(pcm)} bytes")

    wf.close()
    node.send_output("path", pa.array([out_path]))

elif media == "video/raw":
    # Video recording requires ffmpeg subprocess
    # Use ffmpeg-python or subprocess to pipe raw frames into container
    import subprocess
    res = cfg.get("resolution", "1920x1080")
    fps = cfg.get("fps", 30)
    proc = subprocess.Popen([
        "ffmpeg", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24",
        "-s", res, "-r", str(fps), "-i", "pipe:0",
        "-c:v", "libx264", "-preset", "fast", out_path
    ], stdin=subprocess.PIPE, stderr=subprocess.PIPE)

    for event in node:
        if event["type"] != "INPUT" or event["id"] != "data":
            continue
        try:
            blob = event["value"].as_py()
            proc.stdin.write(blob if isinstance(blob, bytes) else bytes(blob))
            print(f"[DM-IO] RECORD mp4 += {len(blob)} bytes")
        except Exception as e:
            print(f"[dm-recorder] Rejected: {e}", file=sys.stderr)

    proc.stdin.close()
    proc.wait()
    node.send_output("path", pa.array([out_path]))

else:
    print(f"[dm-recorder] Unsupported media_type: {media}", file=sys.stderr)
    sys.exit(1)
```

### 依赖

```
soundfile      # (optional, alternative to wave for more formats)
ffmpeg-python  # (optional, for video recording)
```

系统依赖：`ffmpeg`（视频录制时需要）

---

## 三节点对比总览

| | dm-log | dm-save | dm-recorder |
|--|--------|---------|-------------|
| 数据类型 | 文本/结构化 | 二进制 blob | 媒体流 |
| 写入模式 | 追加单文件 | 每事件一个文件 | 追加单容器 |
| 序列化 | UTF-8/JSON/CSV | 无（raw bytes） | 编解码（PCM→wav, raw→mp4） |
| 清理策略 | loguru rotation/retention | max_files/max_size/max_age | max_duration/max_size → 分片 |
| 底层依赖 | `loguru` | 标准库 | `wave` / `ffmpeg` |
| 拒绝条件 | 无法解码为文本 | 无法提取 bytes | 不支持的 media_type |

## 错误处理统一约定

所有存储族节点遇到无法序列化的输入时：
1. **不崩溃**
2. 输出 stderr 错误日志：`[dm-{node}] Rejected: {reason}`
3. 跳过该事件，继续处理后续事件

---

## dm.json 模板汇总

每个节点的 `dm.json` 需要放在 `nodes/dm-{name}/dm.json`，声明 inputs、outputs、config_schema。不在此处列出完整 JSON schema，实现时参考上述 config 表格生成。

---

## 实现顺序建议

1. **dm-log** — 最简单，loguru 做底层，几乎无自研逻辑
2. **dm-save** — 标准库实现，清理逻辑简单
3. **dm-recorder** — 复杂度最高，涉及编解码，优先支持 audio/pcm → wav

## Verification

```bash
# 每个节点独立测试
dm start tests/dataflows/log-test.yml     # dm-log
dm start tests/dataflows/save-test.yml    # dm-save
dm start tests/dataflows/record-test.yml  # dm-recorder

# 检查 runs/:id/out/ 下产物是否正确
# 检查 RuntimeMonitor NodeInspector [DM-IO] 面板是否显示事件
```
