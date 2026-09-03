# rofd Core Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建立可独立构建、安全打开 OFD 文件并通过稳定 `Document`/`Page` Rust API 查询元数据、页数和页面尺寸的 `rofd-core`。

**Architecture:** 保留根包作为渲染原型兼容层，先将 GUI 依赖隔离；新增无 GUI、无 Cairo 依赖的 `rofd-core`。核心把 ZIP 容器、XML 原始模型和公开领域模型分开，页面在 `Document::page()` 时按需解析，所有不可信输入都返回结构化错误而不 panic。

**Tech Stack:** Rust 2021、`zip`、`serde`、`serde-xml-rs`、`thiserror`、Cargo workspace、GitHub Actions。

---

## 范围与文件结构

本计划只覆盖总体设计中的阶段 0 和阶段 1。完成后，现有渲染原型仍在根包 `rofd` 中；后续计划再把显示列表和 Cairo 渲染迁移到 `rofd-render`，因此本计划不改动渲染行为和 Qt UI。

新增文件职责如下：

- `crates/rofd-core/src/error.rs`：稳定错误类别与上下文。
- `crates/rofd-core/src/geometry.rs`：毫米坐标和页面矩形值对象。
- `crates/rofd-core/src/options.rs`：加载模式与 ZIP 资源限制。
- `crates/rofd-core/src/path.rs`：OFD 包内路径规范化与相对解析。
- `crates/rofd-core/src/container.rs`：受限 ZIP 条目索引和读取。
- `crates/rofd-core/src/raw.rs`：不公开的 Serde XML 原始结构。
- `crates/rofd-core/src/document.rs`：公开 `Document`、`Metadata`、`Page` 和 `Warning`。
- `crates/rofd-core/tests/`：以最小内存 OFD 构造器生成的黑盒集成测试。

### Task 1: 隔离现有 GUI 依赖并保存原型基线

**Files:**
- Modify: `Cargo.toml`
- Modify: `.gitignore`
- Create: `tests/fixtures/reference/invoice-current.png`
- Modify: `.github/workflows/rust.yml`

- [ ] **Step 1: 记录当前失败边界**

Run:

```bash
cargo test --all-targets
```

Expected: FAIL；`pkg-config` 报告找不到 `gtk4 >= 4.14` 或 Graphene。把这条结果记录在本任务提交说明中，不把系统包安装作为核心库构建前提。

- [ ] **Step 2: 将废弃 GTK 依赖移出默认构建，并给 Qt 二进制增加 feature gate**

把根 `Cargo.toml` 改为：

```toml
[package]
name = "rofd"
version = "0.1.0"
edition = "2021"
description = "An OFD parser and renderer library."
authors = ["HualetWang <hualet@hualet.org>"]
repository = "https://github.com/hualet/rofd"
license = "MIT"

[features]
default = []
qt-reader = ["dep:qmetaobject"]

[dependencies]
cairo-rs = { version = "0.20.7", features = ["png"] }
env_logger = "0.11.6"
log = { version = "0.4.25", features = ["kv"] }
qmetaobject = { version = "0.2.10", optional = true }
serde = { version = "1.0.217", features = ["derive"] }
serde-xml-rs = "0.6.0"
zip = "2.2.2"

[[bin]]
name = "rofd"
path = "src/bin/rofd/main.rs"
required-features = ["qt-reader"]
```

删除 `[dependencies.gtk]` 段。GTK 代码已经整体注释，`src/bin/rofd/widgets` 也没有被当前二进制引用，不再让它影响库构建。

同时从 `.gitignore` 删除 `Cargo.lock` 这一行。仓库包含可执行程序和 workspace，锁文件必须纳入版本控制，保证 CI 与 Demo 使用相同依赖解析结果。

- [ ] **Step 3: 验证原型库测试可运行**

Run:

```bash
cargo test --lib --tests
```

Expected: PASS，两个现有集成测试完成，`target/out.png` 生成。

- [ ] **Step 4: 保存当前渲染视觉基线**

Run:

```bash
mkdir -p tests/fixtures/reference
cp target/out.png tests/fixtures/reference/invoice-current.png
file tests/fixtures/reference/invoice-current.png
```

Expected: 输出包含 `PNG image data, 400 x 400`。该图片只用于后续人工和像素迁移对照，不在本阶段设为跨机器哈希断言。

- [ ] **Step 5: 让当前 CI 只验证默认核心路径**

把 `.github/workflows/rust.yml` 的 Setup 和验证步骤改为：

```yaml
    - name: Setup
      run: sudo apt-get update && sudo apt-get install -y build-essential libcairo2-dev
    - name: Format
      run: cargo fmt --all -- --check
    - name: Check legacy library
      run: cargo check --lib --tests
    - name: Test
      run: cargo test --lib --tests
```

Expected: YAML 不再安装或要求 GTK；Qt Demo 构建留到 Demo 专用 job。

- [ ] **Step 6: 提交基线隔离**

```bash
git add Cargo.toml Cargo.lock .gitignore .github/workflows/rust.yml tests/fixtures/reference/invoice-current.png
git commit -m "build: isolate GUI dependencies from library"
```

### Task 2: 建立 Cargo workspace 和独立核心 crate

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/rofd-core/Cargo.toml`
- Create: `crates/rofd-core/src/lib.rs`
- Create: `crates/rofd-core/tests/public_api.rs`

- [ ] **Step 1: 写一个失败的核心 crate 黑盒测试**

创建 `crates/rofd-core/tests/public_api.rs`：

```rust
use rofd_core::VERSION;

#[test]
fn exposes_crate_version() {
    assert_eq!(VERSION, "0.2.0");
}
```

- [ ] **Step 2: 验证新包尚不存在**

Run:

```bash
cargo test -p rofd-core --test public_api
```

Expected: FAIL，Cargo 报告 workspace 中不存在 `rofd-core`。

- [ ] **Step 3: 在根包后追加 workspace 配置**

在根 `Cargo.toml` 追加：

```toml
[workspace]
members = [".", "crates/rofd-core"]
default-members = ["crates/rofd-core"]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT"
repository = "https://github.com/hualet/rofd"

[workspace.dependencies]
serde = { version = "1.0.217", features = ["derive"] }
serde-xml-rs = "0.6.0"
thiserror = "2.0.20"
zip = { version = "2.4.2", default-features = false, features = ["deflate"] }
```

- [ ] **Step 4: 创建核心 crate**

创建 `crates/rofd-core/Cargo.toml`：

```toml
[package]
name = "rofd-core"
version = "0.2.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Safe OFD document parsing and query API"

[dependencies]
serde.workspace = true
serde-xml-rs.workspace = true
thiserror.workspace = true
zip.workspace = true

[dev-dependencies]
zip = { workspace = true, features = ["deflate"] }
```

创建 `crates/rofd-core/src/lib.rs`：

```rust
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Safe, read-only OFD document model.

/// The crate API version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

- [ ] **Step 5: 验证独立核心 crate**

Run:

```bash
cargo test -p rofd-core --test public_api
cargo tree -p rofd-core | rg 'gtk|qmetaobject|cairo'
```

Expected: 测试 PASS；第二条命令没有输出并以状态 1 结束，证明核心依赖树不含 GUI 或 Cairo。

- [ ] **Step 6: 提交 workspace 骨架**

```bash
git add Cargo.toml Cargo.lock crates/rofd-core
git commit -m "build: add independent rofd core crate"
```

### Task 3: 定义错误、加载选项和几何值对象

**Files:**
- Create: `crates/rofd-core/src/error.rs`
- Create: `crates/rofd-core/src/geometry.rs`
- Create: `crates/rofd-core/src/options.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Create: `crates/rofd-core/tests/value_types.rs`

- [ ] **Step 1: 写值对象和错误分类测试**

创建 `crates/rofd-core/tests/value_types.rs`：

```rust
use rofd_core::{Error, LoadOptions, Rect, Strictness};

#[test]
fn default_limits_are_finite_and_non_zero() {
    let options = LoadOptions::default();
    assert_eq!(options.strictness, Strictness::Lenient);
    assert!(options.limits.max_entries > 0);
    assert!(options.limits.max_entry_size > 0);
    assert!(options.limits.max_total_size >= options.limits.max_entry_size);
}

#[test]
fn rect_rejects_invalid_numbers() {
    let error = Rect::parse("0 0 NaN 297").unwrap_err();
    assert!(matches!(error, Error::InvalidValue { field: "rectangle", .. }));
}

#[test]
fn rect_requires_exactly_four_values() {
    assert!(Rect::parse("0 0 210").is_err());
    assert!(Rect::parse("0 0 210 297 1").is_err());
}
```

- [ ] **Step 2: 验证测试失败**

Run:

```bash
cargo test -p rofd-core --test value_types
```

Expected: FAIL，导入的公开类型尚不存在。

- [ ] **Step 3: 实现结构化错误**

创建 `crates/rofd-core/src/error.rs`：

```rust
use std::path::PathBuf;

/// Errors returned while loading or querying an OFD document.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Host file I/O failed.
    #[error("I/O error for {path}: {source}")]
    Io {
        /// Host path being accessed.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// The ZIP container is invalid.
    #[error("invalid OFD container: {0}")]
    Container(String),
    /// An XML file cannot be decoded.
    #[error("invalid XML in {path}: {message}")]
    Xml {
        /// Path inside the OFD package.
        path: String,
        /// Parser diagnostic.
        message: String,
    },
    /// A required OFD structure is missing or inconsistent.
    #[error("invalid OFD structure at {path}: {message}")]
    InvalidStructure {
        /// Path inside the OFD package.
        path: String,
        /// Validation diagnostic.
        message: String,
    },
    /// A scalar or compound value has invalid syntax.
    #[error("invalid {field} value `{value}`")]
    InvalidValue {
        /// Field category.
        field: &'static str,
        /// Original value.
        value: String,
    },
    /// A requested package entry does not exist.
    #[error("missing OFD entry: {0}")]
    MissingEntry(String),
    /// A requested page index is outside the document.
    #[error("page index {index} is out of range for {page_count} pages")]
    PageOutOfRange {
        /// Requested zero-based index.
        index: usize,
        /// Available page count.
        page_count: usize,
    },
    /// A configured resource limit was exceeded.
    #[error("resource limit exceeded: {0}")]
    LimitExceeded(String),
    /// The document uses a feature that this version cannot process correctly.
    #[error("unsupported OFD feature: {0}")]
    UnsupportedFeature(String),
}

/// Result type used by rofd-core.
pub type Result<T> = std::result::Result<T, Error>;
```

- [ ] **Step 4: 实现加载选项**

创建 `crates/rofd-core/src/options.rs`：

```rust
/// Handling mode for recoverable conformance problems.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strictness {
    /// Reject every conformance problem encountered by the implemented parser.
    Strict,
    /// Continue past recoverable problems and expose warnings.
    Lenient,
}

/// Limits applied before decompressed package data is accepted.
#[derive(Clone, Debug)]
pub struct ResourceLimits {
    /// Maximum number of ZIP entries.
    pub max_entries: usize,
    /// Maximum uncompressed size of one entry.
    pub max_entry_size: u64,
    /// Maximum sum of declared uncompressed entry sizes.
    pub max_total_size: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            max_entry_size: 64 * 1024 * 1024,
            max_total_size: 512 * 1024 * 1024,
        }
    }
}

/// Options used when opening an OFD package.
#[derive(Clone, Debug)]
pub struct LoadOptions {
    /// Conformance handling mode.
    pub strictness: Strictness,
    /// ZIP resource limits.
    pub limits: ResourceLimits,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            strictness: Strictness::Lenient,
            limits: ResourceLimits::default(),
        }
    }
}
```

- [ ] **Step 5: 实现无 panic 的矩形解析**

创建 `crates/rofd-core/src/geometry.rs`：

```rust
use crate::{Error, Result};

/// A rectangle expressed in OFD millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// Left coordinate.
    pub x: f64,
    /// Top coordinate.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Rect {
    /// Parses four whitespace-separated finite numbers.
    pub fn parse(value: &str) -> Result<Self> {
        let values = value
            .split_whitespace()
            .map(|part| part.parse::<f64>())
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|_| Error::InvalidValue {
                field: "rectangle",
                value: value.to_owned(),
            })?;
        if values.len() != 4 || values.iter().any(|number| !number.is_finite()) {
            return Err(Error::InvalidValue {
                field: "rectangle",
                value: value.to_owned(),
            });
        }
        Ok(Self {
            x: values[0],
            y: values[1],
            width: values[2],
            height: values[3],
        })
    }
}
```

- [ ] **Step 6: 导出公开类型并验证**

把 `crates/rofd-core/src/lib.rs` 改为：

```rust
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Safe, read-only OFD document model.

mod error;
mod geometry;
mod options;

pub use error::{Error, Result};
pub use geometry::Rect;
pub use options::{LoadOptions, ResourceLimits, Strictness};

/// The crate API version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

Run:

```bash
cargo test -p rofd-core --test value_types
cargo clippy -p rofd-core --all-targets -- -D warnings
```

Expected: PASS；Clippy 没有 warning。

- [ ] **Step 7: 提交公共基础类型**

```bash
git add crates/rofd-core/src crates/rofd-core/tests/value_types.rs
git commit -m "feat(core): add load options and value types"
```

### Task 4: 实现安全的 OFD 包内路径

**Files:**
- Create: `crates/rofd-core/src/path.rs`
- Modify: `crates/rofd-core/src/lib.rs`

- [ ] **Step 1: 写路径规范化回归测试**

在即将创建的 `crates/rofd-core/src/path.rs` 底部加入以下单元测试；包内路径是内部安全边界，不进入 v0.2 公共 API：

```rust
#[cfg(test)]
mod tests {
    use super::PackagePath;

    #[test]
    fn resolves_paths_relative_to_declaring_file() {
    let document = PackagePath::new("Doc_0/Document.xml").unwrap();
    assert_eq!(
        document.resolve("Pages/Page_0/Content.xml").unwrap().as_str(),
        "Doc_0/Pages/Page_0/Content.xml"
    );
    }

    #[test]
    fn rejects_absolute_and_escaping_paths() {
    assert!(PackagePath::new("/etc/passwd").is_err());
    assert!(PackagePath::new("../OFD.xml").is_err());
    let document = PackagePath::new("Doc_0/Document.xml").unwrap();
    assert!(document.resolve("../../outside").is_err());
    }

    #[test]
    fn normalizes_dot_and_backslash_separators() {
    assert_eq!(
        PackagePath::new(r"./Doc_0\Pages/Page_0/Content.xml")
            .unwrap()
            .as_str(),
        "Doc_0/Pages/Page_0/Content.xml"
    );
    }
}
```

同时在 `lib.rs` 增加 `mod path;`，使测试编译时真正经过该模块。

- [ ] **Step 2: 验证测试失败**

Run: `cargo test -p rofd-core path::tests`

Expected: FAIL，`path.rs` 和 `PackagePath` 尚未定义。

- [ ] **Step 3: 实现包内路径类型**

创建 `crates/rofd-core/src/path.rs`：

```rust
use crate::{Error, Result};

/// A normalized, relative path inside an OFD package.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PackagePath(String);

impl PackagePath {
    /// Creates a normalized package path.
    pub(crate) fn new(value: &str) -> Result<Self> {
        normalize(value, &[]).map(Self)
    }

    /// Resolves a path relative to the directory containing this entry.
    pub(crate) fn resolve(&self, value: &str) -> Result<Self> {
        let mut base = self.0.split('/').collect::<Vec<_>>();
        base.pop();
        normalize(value, &base).map(Self)
    }

    /// Returns the normalized ZIP entry name.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn normalize(value: &str, base: &[&str]) -> Result<String> {
    let replaced = value.replace('\\', "/");
    if replaced.starts_with('/') {
        return Err(invalid_path(value));
    }
    let mut parts = base.iter().map(|part| (*part).to_owned()).collect::<Vec<_>>();
    for part in replaced.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(invalid_path(value));
                }
            }
            component if component.contains(':') || component.contains('\0') => {
                return Err(invalid_path(value));
            }
            component => parts.push(component.to_owned()),
        }
    }
    if parts.is_empty() {
        return Err(invalid_path(value));
    }
    Ok(parts.join("/"))
}

fn invalid_path(value: &str) -> Error {
    Error::InvalidValue {
        field: "package path",
        value: value.to_owned(),
    }
}
```

在 `lib.rs` 增加私有模块声明：

```rust
mod path;
```

- [ ] **Step 4: 运行路径测试**

Run:

```bash
cargo test -p rofd-core path::tests
```

Expected: 3 tests PASS。

- [ ] **Step 5: 提交路径安全边界**

```bash
git add crates/rofd-core/src/path.rs crates/rofd-core/src/lib.rs
git commit -m "feat(core): normalize OFD package paths"
```

### Task 5: 实现带资源限制的 ZIP 容器

**Files:**
- Create: `crates/rofd-core/src/container.rs`
- Modify: `crates/rofd-core/src/lib.rs`

- [ ] **Step 1: 写内存 ZIP 和限制测试**

在即将创建的 `crates/rofd-core/src/container.rs` 底部加入以下单元测试；`Container` 不进入稳定公共 API：

```rust
#[cfg(test)]
mod tests {
use std::io::{Cursor, Write};

use zip::{write::SimpleFileOptions, ZipWriter};

use super::Container;
use crate::{Error, ResourceLimits};
use crate::path::PackagePath;

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer.start_file(name, SimpleFileOptions::default()).unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn reads_a_normalized_entry() {
    let container = Container::from_bytes(
        archive(&[("OFD.xml", b"<OFD/>")]),
        ResourceLimits::default(),
    )
    .unwrap();
    let contents = container.read(&PackagePath::new("./OFD.xml").unwrap()).unwrap();
    assert_eq!(contents, b"<OFD/>");
}

#[test]
fn rejects_duplicate_normalized_names() {
    let error = Container::from_bytes(
        archive(&[("OFD.xml", b"one"), ("./OFD.xml", b"two")]),
        ResourceLimits::default(),
    )
    .unwrap_err();
    assert!(matches!(error, Error::Container(_)));
}

#[test]
fn rejects_declared_total_size_over_limit() {
    let limits = ResourceLimits {
        max_entries: 10,
        max_entry_size: 4,
        max_total_size: 4,
    };
    let error = Container::from_bytes(archive(&[("OFD.xml", b"12345")]), limits).unwrap_err();
    assert!(matches!(error, Error::LimitExceeded(_)));
}
}
```

同时在 `lib.rs` 增加 `mod container;`，使测试编译时真正经过该模块。

- [ ] **Step 2: 验证测试失败**

Run: `cargo test -p rofd-core container::tests`

Expected: FAIL，`container.rs` 和 `Container` 尚未定义。

- [ ] **Step 3: 实现容器索引和受限读取**

创建 `crates/rofd-core/src/container.rs`：

```rust
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use std::sync::Mutex;

use zip::ZipArchive;

use crate::path::PackagePath;
use crate::{Error, ResourceLimits, Result};

/// A validated, resource-limited OFD ZIP container.
pub(crate) struct Container {
    archive: Mutex<ZipArchive<Cursor<Vec<u8>>>>,
    indexes: HashMap<PackagePath, usize>,
    limits: ResourceLimits,
}

impl std::fmt::Debug for Container {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Container")
            .field("entry_count", &self.indexes.len())
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl Container {
    /// Opens an OFD container from owned bytes.
    pub(crate) fn from_bytes(bytes: Vec<u8>, limits: ResourceLimits) -> Result<Self> {
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|error| Error::Container(error.to_string()))?;
        if archive.len() > limits.max_entries {
            return Err(Error::LimitExceeded(format!(
                "{} entries exceeds {}",
                archive.len(), limits.max_entries
            )));
        }

        let mut indexes = HashMap::new();
        let mut names = HashSet::new();
        let mut total_size = 0_u64;
        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|error| Error::Container(error.to_string()))?;
            let path = PackagePath::new(entry.name())?;
            if !names.insert(path.clone()) {
                return Err(Error::Container(format!(
                    "duplicate normalized entry {}",
                    path.as_str()
                )));
            }
            if entry.size() > limits.max_entry_size {
                return Err(Error::LimitExceeded(format!(
                    "entry {} is {} bytes",
                    path.as_str(), entry.size()
                )));
            }
            total_size = total_size
                .checked_add(entry.size())
                .ok_or_else(|| Error::LimitExceeded("declared ZIP size overflow".to_owned()))?;
            if total_size > limits.max_total_size {
                return Err(Error::LimitExceeded(format!(
                    "declared total size {total_size} exceeds {}",
                    limits.max_total_size
                )));
            }
            indexes.insert(path, index);
        }
        Ok(Self {
            archive: Mutex::new(archive),
            indexes,
            limits,
        })
    }

    /// Reads one validated package entry.
    pub(crate) fn read(&self, path: &PackagePath) -> Result<Vec<u8>> {
        let index = self
            .indexes
            .get(path)
            .copied()
            .ok_or_else(|| Error::MissingEntry(path.as_str().to_owned()))?;
        let mut archive = self
            .archive
            .lock()
            .map_err(|_| Error::Container("ZIP archive lock is poisoned".to_owned()))?;
        let mut entry = archive
            .by_index(index)
            .map_err(|error| Error::Container(error.to_string()))?;
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry
            .by_ref()
            .take(self.limits.max_entry_size + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| Error::Container(error.to_string()))?;
        if bytes.len() as u64 > self.limits.max_entry_size {
            return Err(Error::LimitExceeded(format!(
                "entry {} exceeded {} bytes while reading",
                path.as_str(), self.limits.max_entry_size
            )));
        }
        Ok(bytes)
    }
}
```

在 `lib.rs` 增加私有模块声明：

```rust
mod container;
```

- [ ] **Step 4: 验证容器行为**

Run:

```bash
cargo test -p rofd-core container::tests
```

Expected: 3 tests PASS。

- [ ] **Step 5: 提交安全容器**

```bash
git add crates/rofd-core/src/container.rs crates/rofd-core/src/lib.rs
git commit -m "feat(core): add resource-limited OFD container"
```

### Task 6: 解析 OFD 入口和文档页面索引

**Files:**
- Create: `crates/rofd-core/src/raw.rs`
- Create: `crates/rofd-core/src/document.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Create: `crates/rofd-core/tests/support/mod.rs`
- Create: `crates/rofd-core/tests/document_open.rs`

- [ ] **Step 1: 创建集成测试 OFD 构造器**

创建 `crates/rofd-core/tests/support/mod.rs`：

```rust
use std::io::{Cursor, Write};

use zip::{write::SimpleFileOptions, ZipWriter};

pub fn ofd_with_doc_bodies(page_xml: &str, doc_body_count: usize) -> Vec<u8> {
    let body = r#"<ofd:DocBody>
    <ofd:DocInfo>
      <ofd:DocID>fixture-id</ofd:DocID>
      <ofd:Title>Fixture</ofd:Title>
      <ofd:Creator>rofd tests</ofd:Creator>
    </ofd:DocInfo>
    <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
  </ofd:DocBody>"#;
    let ofd_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016" DocType="OFD" Version="1.0">{}</ofd:OFD>"#,
        body.repeat(doc_body_count)
    );
    let entries = [
        ("OFD.xml", ofd_xml.as_str()),
        (
            "Doc_0/Document.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea>
    <ofd:MaxUnitID>2</ofd:MaxUnitID>
  </ofd:CommonData>
  <ofd:Pages><ofd:Page ID="2" BaseLoc="Pages/Page_0/Content.xml"/></ofd:Pages>
</ofd:Document>"#,
        ),
        ("Doc_0/Pages/Page_0/Content.xml", page_xml),
    ];
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer.start_file(name, SimpleFileOptions::default()).unwrap();
        writer.write_all(contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

pub fn minimal_ofd(page_xml: &str) -> Vec<u8> {
    ofd_with_doc_bodies(page_xml, 1)
}

pub const PAGE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:Area>
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;
```

- [ ] **Step 2: 写打开文档的黑盒测试**

创建 `crates/rofd-core/tests/document_open.rs`：

```rust
mod support;

use rofd_core::{Document, Error, LoadOptions};
use support::{minimal_ofd, PAGE_XML};

#[test]
fn opens_document_and_exposes_metadata_and_page_count() {
    let document = Document::from_bytes(minimal_ofd(PAGE_XML), LoadOptions::default()).unwrap();
    assert_eq!(document.page_count(), 1);
    assert_eq!(document.metadata().document_id.as_deref(), Some("fixture-id"));
    assert_eq!(document.metadata().title.as_deref(), Some("Fixture"));
    assert_eq!(document.metadata().creator.as_deref(), Some("rofd tests"));
}

#[test]
fn missing_entry_point_is_reported() {
    let error = Document::from_bytes(Vec::new(), LoadOptions::default()).unwrap_err();
    assert!(matches!(error, Error::Container(_)));
}

#[test]
fn multiple_doc_bodies_are_explicitly_unsupported_in_v02() {
    let error = Document::from_bytes(
        support::ofd_with_doc_bodies(PAGE_XML, 2),
        LoadOptions::default(),
    )
    .unwrap_err();
    assert!(matches!(error, Error::UnsupportedFeature(_)));
}
```

- [ ] **Step 3: 验证文档 API 测试失败**

Run: `cargo test -p rofd-core --test document_open`

Expected: FAIL，`Document` 尚未定义。

- [ ] **Step 4: 实现内部 XML 原始类型**

创建 `crates/rofd-core/src/raw.rs`：

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct OfdRoot {
    #[serde(rename = "DocBody")]
    pub(crate) doc_bodies: Vec<DocBody>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DocBody {
    pub(crate) doc_info: DocInfo,
    pub(crate) doc_root: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DocInfo {
    #[serde(rename = "DocID")]
    pub(crate) document_id: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) subject: Option<String>,
    #[serde(rename = "Abstract")]
    pub(crate) abstract_: Option<String>,
    pub(crate) creator: Option<String>,
    pub(crate) creator_version: Option<String>,
    pub(crate) creation_date: Option<String>,
    pub(crate) mod_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DocumentRoot {
    pub(crate) common_data: CommonData,
    pub(crate) pages: PageList,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct CommonData {
    pub(crate) page_area: PageArea,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PageArea {
    pub(crate) physical_box: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PageList {
    #[serde(rename = "Page")]
    pub(crate) pages: Vec<PageEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PageEntry {
    #[serde(rename = "ID")]
    pub(crate) id: u64,
    #[serde(rename = "BaseLoc")]
    pub(crate) base_loc: String,
}

```

- [ ] **Step 5: 实现 Document 打开和页面索引**

创建 `crates/rofd-core/src/document.rs`：

```rust
use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde::de::DeserializeOwned;

use crate::raw::{DocumentRoot, OfdRoot};
use crate::container::Container;
use crate::path::PackagePath;
use crate::{Error, LoadOptions, Result};

/// Descriptive metadata stored in an OFD document.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Metadata {
    /// Document identifier.
    pub document_id: Option<String>,
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Document abstract.
    pub abstract_text: Option<String>,
    /// Producing application.
    pub creator: Option<String>,
    /// Producing application version.
    pub creator_version: Option<String>,
    /// Original creation date as stored by the producer.
    pub creation_date: Option<String>,
    /// Modification date as stored by the producer.
    pub modification_date: Option<String>,
}

#[derive(Debug)]
struct PageReference {
    id: u64,
    path: PackagePath,
}

#[derive(Debug)]
struct DocumentInner {
    container: Container,
    metadata: Metadata,
    default_page_area: crate::raw::PageArea,
    pages: Vec<PageReference>,
}

/// A read-only OFD document.
#[derive(Clone, Debug)]
pub struct Document(Arc<DocumentInner>);

impl Document {
    /// Opens an OFD document from a host file path.
    pub fn open(path: impl AsRef<Path>, options: LoadOptions) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_bytes(bytes, options)
    }

    /// Opens an OFD document from owned bytes.
    pub fn from_bytes(bytes: Vec<u8>, options: LoadOptions) -> Result<Self> {
        let container = Container::from_bytes(bytes, options.limits)?;
        let entry_path = PackagePath::new("OFD.xml")?;
        let ofd: OfdRoot = parse_xml(&container, &entry_path)?;
        if ofd.doc_bodies.len() != 1 {
            return Err(Error::UnsupportedFeature(format!(
                "v0.2 requires exactly one DocBody, found {}",
                ofd.doc_bodies.len()
            )));
        }
        let body = ofd.doc_bodies.into_iter().next().ok_or_else(|| {
            Error::InvalidStructure {
                path: entry_path.as_str().to_owned(),
                message: "DocBody is missing".to_owned(),
            }
        })?;
        let document_path = entry_path.resolve(&body.doc_root)?;
        let root: DocumentRoot = parse_xml(&container, &document_path)?;
        let pages = root
            .pages
            .pages
            .into_iter()
            .map(|page| {
                Ok(PageReference {
                    id: page.id,
                    path: document_path.resolve(&page.base_loc)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let info = body.doc_info;
        Ok(Self(Arc::new(DocumentInner {
            container,
            metadata: Metadata {
                document_id: info.document_id,
                title: info.title,
                author: info.author,
                subject: info.subject,
                abstract_text: info.abstract_,
                creator: info.creator,
                creator_version: info.creator_version,
                creation_date: info.creation_date,
                modification_date: info.mod_date,
            },
            default_page_area: root.common_data.page_area,
            pages,
        })))
    }

    /// Returns document metadata.
    pub fn metadata(&self) -> &Metadata {
        &self.0.metadata
    }

    /// Returns the number of indexed pages.
    pub fn page_count(&self) -> usize {
        self.0.pages.len()
    }
}

fn parse_xml<T: DeserializeOwned>(container: &Container, path: &PackagePath) -> Result<T> {
    let bytes = container.read(path)?;
    serde_xml_rs::from_reader(bytes.as_slice()).map_err(|error| Error::Xml {
        path: path.as_str().to_owned(),
        message: error.to_string(),
    })
}
```

在 `lib.rs` 增加：

```rust
mod document;
mod raw;
pub use document::{Document, Metadata};
```

- [ ] **Step 6: 运行文档打开测试并提交**

Run:

```bash
cargo test -p rofd-core --test document_open
```

Expected: 3 tests PASS。页面引用会在下一任务开始被读取和缓存，届时再启用 `-D warnings` 门槛。

```bash
git add crates/rofd-core/src crates/rofd-core/tests
git commit -m "feat(core): open OFD documents and index pages"
```

### Task 7: 提供按索引访问的 Page API

**Files:**
- Modify: `crates/rofd-core/src/document.rs`
- Modify: `crates/rofd-core/src/raw.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Create: `crates/rofd-core/tests/page_access.rs`

- [ ] **Step 1: 写页面访问测试**

创建 `crates/rofd-core/tests/page_access.rs`：

```rust
mod support;

use rofd_core::{Document, Error, LoadOptions, Rect};
use support::{minimal_ofd, PAGE_XML};

#[test]
fn loads_page_size_and_identity() {
    let document = Document::from_bytes(minimal_ofd(PAGE_XML), LoadOptions::default()).unwrap();
    let page = document.page(0).unwrap();
    assert_eq!(page.index(), 0);
    assert_eq!(page.object_id(), 2);
    assert_eq!(
        page.size(),
        Rect { x: 0.0, y: 0.0, width: 210.0, height: 297.0 }
    );
}

#[test]
fn page_uses_document_area_when_page_area_is_absent() {
    let page_without_area = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;
    let document =
        Document::from_bytes(minimal_ofd(page_without_area), LoadOptions::default()).unwrap();
    assert_eq!(document.page(0).unwrap().size().width, 210.0);
}

#[test]
fn reports_out_of_range_page() {
    let document = Document::from_bytes(minimal_ofd(PAGE_XML), LoadOptions::default()).unwrap();
    let error = document.page(1).unwrap_err();
    assert!(matches!(error, Error::PageOutOfRange { index: 1, page_count: 1 }));
}
```

- [ ] **Step 2: 验证 Page API 测试失败**

Run: `cargo test -p rofd-core --test page_access`

Expected: FAIL，`Document::page` 尚未定义。

- [ ] **Step 3: 定义页面原始结构并让页面面积可克隆**

在 `raw.rs` 的 `PageArea` 上增加 `Clone`：

```rust
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PageArea {
    pub(crate) physical_box: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PageRoot {
    pub(crate) area: Option<PageArea>,
}
```

- [ ] **Step 4: 实现拥有型 Page**

把 `document.rs` 的同步导入改为：

```rust
use std::sync::{Arc, OnceLock};
```

给 `PageReference` 增加页面缓存，并在创建引用时初始化：

```rust
#[derive(Debug)]
struct PageReference {
    id: u64,
    path: PackagePath,
    cache: OnceLock<Arc<PageData>>,
}
```

```rust
                Ok(PageReference {
                    id: page.id,
                    path: document_path.resolve(&page.base_loc)?,
                    cache: OnceLock::new(),
                })
```

在 `document.rs` 中加入：

```rust
#[derive(Debug)]
struct PageData {
    size: crate::Rect,
}

/// One parsed page in an OFD document.
#[derive(Clone, Debug)]
pub struct Page {
    _document: Arc<DocumentInner>,
    index: usize,
    object_id: u64,
    data: Arc<PageData>,
}

impl Page {
    /// Returns the zero-based page index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the OFD object identifier of the page.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Returns the effective physical page box in millimetres.
    pub fn size(&self) -> crate::Rect {
        self.data.size
    }

}
```

在 `impl Document` 中加入：

```rust
    /// Loads and returns a page by zero-based index.
    pub fn page(&self, index: usize) -> Result<Page> {
        let reference = self.0.pages.get(index).ok_or(Error::PageOutOfRange {
            index,
            page_count: self.page_count(),
        })?;
        if let Some(data) = reference.cache.get() {
            return Ok(Page {
                _document: Arc::clone(&self.0),
                index,
                object_id: reference.id,
                data: Arc::clone(data),
            });
        }
        let page: crate::raw::PageRoot = parse_xml(&self.0.container, &reference.path)?;
        let area = page.area.unwrap_or_else(|| self.0.default_page_area.clone());
        let size = crate::Rect::parse(&area.physical_box)?;
        let parsed = Arc::new(PageData { size });
        let data = reference.cache.get_or_init(|| Arc::clone(&parsed));
        Ok(Page {
            _document: Arc::clone(&self.0),
            index,
            object_id: reference.id,
            data: Arc::clone(data),
        })
    }
```

把 `lib.rs` 的导出改为：

```rust
pub use document::{Document, Metadata, Page};
```

`_document` 让 Page 拥有底层文档资源的生命周期，`OnceLock` 保证顺序访问时页面 XML 只解析一次；下一个渲染计划会扩充 `PageData`，不向公共 API 暴露 ZIP 数据。

- [ ] **Step 5: 验证页面访问和所有既有核心测试**

Run:

```bash
cargo test -p rofd-core
cargo clippy -p rofd-core --all-targets -- -D warnings
```

Expected: 全部 PASS；没有 panic 或 dead-code warning，不允许用 `#[allow(dead_code)]` 掩盖未使用实现。

- [ ] **Step 6: 提交 Page API**

```bash
git add crates/rofd-core/src crates/rofd-core/tests/page_access.rs
git commit -m "feat(core): expose lazy page access"
```

### Task 8: 增加严格/宽容模式和可查询 warnings

**Files:**
- Modify: `crates/rofd-core/src/document.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Create: `crates/rofd-core/tests/strictness.rs`

- [ ] **Step 1: 写页面回退的模式测试**

创建 `crates/rofd-core/tests/strictness.rs`：

```rust
mod support;

use rofd_core::{Document, Error, LoadOptions, Strictness, WarningCode};
use support::minimal_ofd;

const PAGE_WITHOUT_AREA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Content><ofd:Layer ID="1"/></ofd:Content>
</ofd:Page>"#;

#[test]
fn lenient_mode_falls_back_and_records_warning() {
    let document = Document::from_bytes(minimal_ofd(PAGE_WITHOUT_AREA), LoadOptions::default()).unwrap();
    document.page(0).unwrap();
    document.page(0).unwrap();
    assert_eq!(document.warnings().len(), 1);
    assert_eq!(document.warnings()[0].code, WarningCode::PageAreaFallback);
}

#[test]
fn strict_mode_rejects_missing_page_area() {
    let options = LoadOptions {
        strictness: Strictness::Strict,
        ..LoadOptions::default()
    };
    let document = Document::from_bytes(minimal_ofd(PAGE_WITHOUT_AREA), options).unwrap();
    let error = document.page(0).unwrap_err();
    assert!(matches!(error, Error::InvalidStructure { .. }));
}
```

- [ ] **Step 2: 验证模式测试失败**

Run: `cargo test -p rofd-core --test strictness`

Expected: FAIL，warnings API 尚不存在。

- [ ] **Step 3: 实现 warning 模型并保存 strictness**

把 `use std::sync::{Arc, OnceLock};` 改为 `use std::sync::{Arc, Mutex, OnceLock};`，并在 `document.rs` 加入：

```rust
/// Stable categories for recoverable OFD problems.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WarningCode {
    /// The page omitted Area and inherited the document PageArea.
    PageAreaFallback,
}

/// A recoverable OFD conformance diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Warning {
    /// Machine-readable category.
    pub code: WarningCode,
    /// Path inside the OFD package.
    pub path: String,
    /// Human-readable explanation.
    pub message: String,
}
```

把 `DocumentInner` 增加：

```rust
    strictness: crate::Strictness,
    warnings: Mutex<Vec<Warning>>,
```

把 `Document::from_bytes` 的第一条容器构造语句改为：

```rust
        let strictness = options.strictness;
        let container = Container::from_bytes(bytes, options.limits)?;
```

构造 `DocumentInner` 时增加：

```rust
            strictness,
            warnings: Mutex::new(Vec::new()),
```

- [ ] **Step 4: 实现模式分支和 warning 查询**

在 `Document::page` 解析页面后，用以下逻辑替换无条件回退：

```rust
        let area = match page.area {
            Some(area) => area,
            None if self.0.strictness == crate::Strictness::Strict => {
                return Err(Error::InvalidStructure {
                    path: reference.path.as_str().to_owned(),
                    message: "Page.Area is missing".to_owned(),
                });
            }
            None => {
                self.0
                    .warnings
                    .lock()
                    .map_err(|_| Error::InvalidStructure {
                        path: reference.path.as_str().to_owned(),
                        message: "warning store lock is poisoned".to_owned(),
                    })?
                    .push(Warning {
                        code: WarningCode::PageAreaFallback,
                        path: reference.path.as_str().to_owned(),
                        message: "Page.Area is missing; inherited Document PageArea".to_owned(),
                    });
                self.0.default_page_area.clone()
            }
        };
```

在 `impl Document` 增加：

```rust
    /// Returns a snapshot of recoverable diagnostics collected so far.
    pub fn warnings(&self) -> Vec<Warning> {
        self.0
            .warnings
            .lock()
            .map(|warnings| warnings.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }
```

在 `lib.rs` 导出：

```rust
pub use document::{Document, Metadata, Page, Warning, WarningCode};
```

- [ ] **Step 5: 验证两种模式并提交**

Run:

```bash
cargo test -p rofd-core --test strictness
cargo test -p rofd-core
```

Expected: 全部 PASS。

```bash
git add crates/rofd-core/src crates/rofd-core/tests/strictness.rs
git commit -m "feat(core): report recoverable OFD warnings"
```

### Task 9: 用仓库真实样例验证公开 API

**Files:**
- Create: `crates/rofd-core/tests/real_fixture.rs`

- [ ] **Step 1: 写真实发票黑盒测试**

创建 `crates/rofd-core/tests/real_fixture.rs`：

```rust
use std::path::PathBuf;

use rofd_core::{Document, LoadOptions, Rect};

#[test]
fn opens_repository_invoice_fixture() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../learning/test.ofd");
    let document = Document::open(path, LoadOptions::default()).unwrap();
    assert_eq!(document.page_count(), 1);
    assert_eq!(document.metadata().document_id.as_deref(), Some("2195d5df959c419cb575dab5eeabb065"));
    assert_eq!(
        document.page(0).unwrap().size(),
        Rect { x: 0.0, y: 0.0, width: 211.5, height: 140.0 }
    );
}
```

- [ ] **Step 2: 运行测试并定位真实生产者差异**

Run:

```bash
cargo test -p rofd-core --test real_fixture -- --nocapture
```

Expected: PASS。若 Serde 字段命名、空节点或 XML namespace 导致失败，只修改 `raw.rs` 的反序列化标注和可选性；不得为该文件硬编码路径或字段值。

- [ ] **Step 3: 运行完整核心质量门槛**

Run:

```bash
cargo fmt --all -- --check
cargo clippy -p rofd-core --all-targets -- -D warnings
cargo test -p rofd-core
```

Expected: 三条命令全部状态 0。

- [ ] **Step 4: 提交真实语料验证**

```bash
git add crates/rofd-core/tests/real_fixture.rs crates/rofd-core/src/raw.rs
git commit -m "test(core): cover repository OFD fixture"
```

### Task 10: 文档化核心 API 并修正 CI 分层

**Files:**
- Create: `crates/rofd-core/README.md`
- Modify: `README.md`
- Modify: `.github/workflows/rust.yml`

- [ ] **Step 1: 写可编译的 README 示例**

创建 `crates/rofd-core/README.md`：

````markdown
# rofd-core

`rofd-core` provides a safe, read-only OFD document model without GUI or
rendering dependencies.

```rust
use rofd_core::{Document, LoadOptions};

let document = Document::open("document.ofd", LoadOptions::default())?;
println!("{} pages", document.page_count());
let first_page = document.page(0)?;
println!("{} mm × {} mm", first_page.size().width, first_page.size().height);
# Ok::<(), rofd_core::Error>(())
```

The v0.2 foundation supports one `DocBody`. Multiple document bodies return an
explicit `UnsupportedFeature` error. Rendering, text extraction, annotations,
signatures and the C ABI are delivered by the following implementation phases.
````

将该 README 内容同时放进 `lib.rs` crate 文档，或通过 `#![doc = include_str!("../README.md")]` 引入；如果使用 include，则删除原有 `//!`，避免重复 crate 文档属性。

- [ ] **Step 2: 更新根 README 的状态与构建方式**

把 README 的 Plan 和 Usage 改成两条明确路径：

````markdown
## Project status

- `rofd-core`: safe OFD container, metadata and page access foundation.
- root `rofd` package: legacy Cairo rendering prototype kept during migration.
- Qt/QML reader: design approved; implementation follows the C ABI phase.

## Build the core library

```bash
cargo test -p rofd-core
```

## Run the legacy Qt prototype

```bash
cargo run --features qt-reader --bin rofd
```
````

保留 reference projects 和 license；删除“GUI application 已完成”以及默认 `cargo run` 能启动阅读器的表述。

- [ ] **Step 3: 将 CI 默认门槛切到核心 crate**

把 `.github/workflows/rust.yml` 的验证命令更新为：

```yaml
    - name: Format
      run: cargo fmt --all -- --check
    - name: Lint core
      run: cargo clippy -p rofd-core --all-targets -- -D warnings
    - name: Test core
      run: cargo test -p rofd-core
```

根渲染原型暂不作为默认 member，但保留 Task 1 已验证的 `cargo test --lib --tests` 本地基线命令。后续 `rofd-render` 迁移完成时再恢复全 workspace CI。

- [ ] **Step 4: 验证文档测试和 workspace 默认行为**

Run:

```bash
cargo test --doc -p rofd-core
cargo test
cargo tree -p rofd-core | rg 'gtk|qmetaobject|cairo'
git diff --check
```

Expected: 前两条 PASS；依赖检查无匹配；`git diff --check` 无输出。

- [ ] **Step 5: 提交文档和 CI**

```bash
git add README.md crates/rofd-core/README.md crates/rofd-core/src/lib.rs .github/workflows/rust.yml
git commit -m "docs: describe rofd core foundation"
```

## 计划完成验收

执行：

```bash
cargo fmt --all -- --check
cargo clippy -p rofd-core --all-targets -- -D warnings
cargo test -p rofd-core
cargo test --doc -p rofd-core
cargo tree -p rofd-core | rg 'gtk|qmetaobject|cairo'
git status --short
```

验收结果必须满足：

- 前四条测试/检查命令状态为 0。
- 依赖树检查没有输出。
- `git status --short` 没有输出。
- `Document::open` 和 `Document::from_bytes` 能打开测试 OFD。
- `Document::metadata`、`page_count`、`page` 和 `warnings` 有黑盒测试。
- 页面越界、非法路径、重复条目、资源超限、多 `DocBody` 和严格模式均返回明确错误。
- 核心 crate 不依赖 Cairo、Qt 或 GTK。

完成此计划后，下一份计划是 `rofd-render`：先定义显示列表，再迁移现有 Cairo 渲染，并以 `tests/fixtures/reference/invoice-current.png` 保持视觉连续性。
