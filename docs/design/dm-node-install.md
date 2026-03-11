# dm node install 分发策略

## 现状

`dm node install` 目前支持两种构建方式，均为本地编译：

- **Python 节点**：`uv pip install` → 创建 venv → 生成可执行文件
- **Rust 节点**：`cargo install` → 本地编译 → 生成二进制

两种方式都要求用户已安装对应工具链，且首次编译较慢（Rust 可达 2-5 分钟）。

## 目标

参考 `cargo-binstall` 的策略：**优先下载预编译二进制，没有则 fallback 到本地编译**。

```
dm node install dm-queue
  → 检查 GitHub Release 是否有当前平台的预编译二进制
  → 有 → 下载，秒级完成
  → 没有 → fallback 到 cargo install，本地编译
```

## 参考：cargo-binstall 的做法

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) 是 `cargo install` 的增强版：

1. 从 crate metadata 读取二进制下载地址（`[package.metadata.binstall]`）
2. 默认去 GitHub Releases 找 `{name}-{version}-{target}.tar.gz`
3. 校验 checksum
4. 找不到预编译版本 → fallback 到 `cargo install`

用户的体验：

```bash
# cargo-binstall 方式（秒级）
cargo binstall ripgrep    # 下载预编译

# cargo install 方式（分钟级）
cargo install ripgrep     # 本地编译
```

## dm node install 改造方案

### dm.json 扩展

在 `source` 字段中新增 `binary` 分发信息：

```json
{
  "id": "dm-queue",
  "source": {
    "build": "cargo install",
    "binary": {
      "github": "l1veIn/dora-manager",
      "asset_pattern": "dm-queue-{version}-{target}.tar.gz"
    }
  }
}
```

### 安装流程

```
dm node install <id>
  │
  ├─ 1. 读取 dm.json
  ├─ 2. source.binary 存在？
  │     ├─ YES → 检测当前平台 target triple
  │     │        → 到 GitHub Releases 查找匹配的 asset
  │     │        → 下载 + 校验 + 解压到 nodes/<id>/bin/
  │     │        → 成功？ → 完成
  │     │        → 失败？ → fallback 到 step 3
  │     └─ NO  → step 3
  └─ 3. 走现有的本地编译路线（cargo install / uv pip install）
```

### 平台检测

| target triple | 说明 |
|---|---|
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-unknown-linux-gnu` | Linux x86_64 |
| `aarch64-unknown-linux-gnu` | Linux ARM64 |

Rust 的 `std::env::consts::{OS, ARCH}` 可直接获取。

### CI 构建（GitHub Actions）

```yaml
# .github/workflows/release-nodes.yml
strategy:
  matrix:
    include:
      - target: x86_64-apple-darwin
        os: macos-latest
      - target: aarch64-apple-darwin
        os: macos-latest
      - target: x86_64-unknown-linux-gnu
        os: ubuntu-latest
      - target: aarch64-unknown-linux-gnu
        os: ubuntu-latest  # cross-compile

steps:
  - run: cargo build --release --target ${{ matrix.target }}
  - run: tar czf dm-queue-$VERSION-${{ matrix.target }}.tar.gz -C target/${{ matrix.target }}/release dm-queue
  # Upload to GitHub Release
```

## 分阶段落地

| 阶段 | 做什么 | 用户体验 |
|---|---|---|
| **现在** | 纯 `cargo install` / `uv pip install` | 需要工具链，能跑 |
| **Phase 1** | GitHub Actions 多平台 CI → Release assets | 构建侧就绪 |
| **Phase 2** | `install_node` 加预编译下载逻辑 + fallback | 秒级安装，无工具链要求 |
| **Phase 3** | Python 节点也可用 PyInstaller 预编译单文件 | 统一体验 |
