# rofd 类 Poppler 阅读库与 Demo 阅读器设计

## 1. 背景与目标

rofd 当前已经能打开一个 OFD ZIP 包，反序列化部分 XML 节点，并用 Cairo 绘制示例发票中的文字、图像和简单图形。现有实现证明了 Rust 解析和 Cairo 渲染路线可行，但整体仍是面向单一样例的原型，尚不具备供其他应用长期依赖的库接口。

本轮建设目标是把项目演进为一个类似 Poppler 使用方式的只读 OFD 阅读组件：调用者通过稳定的文档和页面对象读取内容，按页渲染或查询文本；同时提供稳定 C ABI，并用 Qt/QML 实现一个只通过公共接口访问 OFD 内容的 Demo 阅读器。

首个可交付版本命名为 v0.2，定位为“可嵌入的只读 OFD 阅读库”。它不是完整 OFD 编辑器，也不承诺首版覆盖 GB/T 33190-2016 的全部写入、签章创建和加密能力。

## 2. 当前状态与差距

### 2.1 已有基础

- `read_ofd` 可以从路径打开 ZIP 包并读取 `OFD.xml`。
- 已定义部分文档、页面、资源、注释、文字、图像和图形 XML 数据结构。
- 可以遍历文档页面，并把部分页面内容绘制到 Cairo context。
- 示例发票中的 `TextObject`、`ImageObject`、`PathObject` 和嵌套 `PageBlock` 已有初步实现。
- 有一个导出 PNG 的 smoke test 和一个读取 smoke test。
- 仓库已尝试 GTK 阅读器和 Qt/QML 阅读器两条 GUI 路线。

### 2.2 与阅读库目标的主要差距

| 能力 | 当前实现 | v0.2 目标 |
| --- | --- | --- |
| 公共对象模型 | 三个顶层函数，内部模块未公开 | `Document`、`Page`、元数据、文字、注释和签名的稳定 Rust API |
| 输入 | 文件路径 | 路径和内存字节；内部为可扩展数据源抽象 |
| 页面访问 | 渲染函数内部遍历全部页面 | 页数、按索引取页、页面尺寸、按页渲染 |
| 架构 | ZIP、XML、资源和渲染互相持有可变状态 | 容器、解析模型、文档模型、显示列表和渲染后端分层 |
| 图形 | 用 `Boundary` 画矩形，没有解析路径数据 | 路径指令、填充/描边、线型、裁剪、透明度和 CTM |
| 文字 | 整段 `show_text`，忽略字形定位 | `DeltaX`/`DeltaY`、字形变换、嵌入字体、文字区域 |
| 图像 | 假定 PNG，资源访问包含 `unwrap` | PNG/JPEG、CTM、裁剪、透明度、缓存和显式错误 |
| 页面结构 | 单层、单页样例 | 多页、模板页、前景/正文/背景层和页面绘制参数 |
| 查询 | 无 | 元数据、文字提取、文本查找、注释列表、签名列表 |
| C ABI | 无 | 不透明句柄、明确所有权、错误码和版本化头文件 |
| Demo | QML 占位窗口 | 打开、多页浏览、缩放、适宽、跳页、搜索和文档信息 |
| 错误处理 | `Box<dyn Error>`、`unwrap`、`assert` | 结构化错误；不可信输入不能导致 panic |
| 构建 | GTK 与 Qt 都是核心包的强制依赖 | 核心、C ABI、CLI 和 Qt Demo 独立构建 |
| 验证 | 单一样例、无断言式渲染验证 | 多厂商语料、解析断言、像素基准、畸形输入和 ABI 测试 |

### 2.3 标准覆盖差距

GB/T 33190-2016 除基础文件结构外，还定义了页对象、大纲、资源、页面坐标和绘制参数、颜色空间与渐变、裁剪区、路径、图像、文字、视频、复合对象、动作、注释、自定义标引、扩展信息、数字签名、版本和附件。

v0.2 聚焦阅读所需的基础结构、资源、页面、路径、图像、文字、模板、注释外观以及签名信息。视频、复杂动作、版本切换、附件提取、权限执行、解密、签名创建和编辑接口不进入 v0.2；遇到这些内容时必须返回可识别的“不支持”信息，不能静默误渲染。

## 3. 方案选择

### 3.1 采用渐进式多 crate 重构

保留已经验证过的 Rust、Serde XML、ZIP 和 Cairo 技术路线，但不保留现有模块之间的耦合关系。先建立公共接口与测试边界，再把现有解析和渲染逻辑逐项迁移进去。

该方案相对从零重写能够持续保留可运行成果；相对封装其他 OFD 实现，不会引入 Java/C++ 运行时、额外许可证和部署依赖。

### 3.2 工作区结构

```text
Cargo.toml
crates/
  rofd-core/       OFD 容器、XML 解析、文档模型和查询
  rofd-render/     显示列表、资源解析和 Cairo 渲染后端
  rofd-ffi/        稳定 C ABI 与公开头文件
apps/
  rofd-reader/     Qt/QML Demo 阅读器
tools/
  rofd-cli/        文档检查、信息输出和页面导出
tests/
  fixtures/        合法、边界、损坏和多厂商测试文件
```

依赖方向固定为：`rofd-render -> rofd-core`、`rofd-ffi -> rofd-core + rofd-render`、`rofd-cli -> rofd-core + rofd-render`、`rofd-reader -> rofd-ffi`。核心 crate 不依赖 GTK、Qt、QML 或任何桌面框架。

Demo 强制通过 C ABI 使用库，用它持续验证外部消费者真正能完成阅读器功能，而不是依赖 Rust 私有实现。Rust 原生调用示例和测试则直接使用 Rust API。

## 4. 公共 API 设计

### 4.1 Rust API

Rust API 使用拥有型 `Document` 和借用文档资源的 `Page<'_>`：

```rust
use rofd_core::{Document, LoadOptions};
use rofd_render::{CairoRenderer, RenderOptions};

let document = Document::open("invoice.ofd", LoadOptions::default())?;
println!("pages: {}", document.page_count());

let page = document.page(0)?;
let size = page.size();
let text = page.text()?;

let renderer = CairoRenderer::new();
renderer.render_page(&page, &mut cairo_context, &RenderOptions::default())?;
```

v0.2 的稳定能力集合为：

- `Document::open` 和 `Document::from_bytes`。
- `Document::metadata`、`page_count`、`page`、`permissions`、`signatures`。
- `Page::index`、`size`、`label`、`text`、`find_text`、`annotations`。
- `CairoRenderer::render_page` 和 `RasterRenderer::render_page`。
- `RenderOptions` 支持 DPI、缩放、旋转、背景色和裁剪区域。

解析层的原始 XML 类型不属于稳定公共 API。调用者只能通过领域对象访问内容，避免标准字段变化直接破坏 ABI。

### 4.2 C ABI

C ABI 使用 `rofd_` 前缀和不透明句柄：

```c
typedef struct rofd_document rofd_document_t;
typedef struct rofd_page rofd_page_t;
typedef struct rofd_error rofd_error_t;

rofd_status_t rofd_document_open(
    const char *path,
    const rofd_load_options_t *options,
    rofd_document_t **document,
    rofd_error_t **error);

size_t rofd_document_get_n_pages(const rofd_document_t *document);

rofd_status_t rofd_document_get_page(
    const rofd_document_t *document,
    size_t index,
    rofd_page_t **page,
    rofd_error_t **error);

rofd_status_t rofd_page_render_cairo(
    const rofd_page_t *page,
    cairo_t *context,
    const rofd_render_options_t *options,
    rofd_error_t **error);
```

所有创建函数都有对应的 `*_free`。字符串要么由调用者提供缓冲区，要么返回带专用释放函数的库所有内存；禁止跨 ABI 直接暴露 Rust `String`、`Vec`、引用或枚举布局。所有 panic 必须在 FFI 边界捕获并转换为内部错误。

C 头文件公开 `ROFD_ABI_VERSION`，新增能力优先通过带 `struct_size` 的 options 结构扩展。v0.2 发布后，不破坏已公开函数和结构字段的二进制兼容性。

## 5. 内部组件与数据流

### 5.1 安全容器

`Container` 负责 ZIP 条目索引、路径规范化和按需读取。它拒绝绝对路径、父目录逃逸、重复关键入口、超过配置阈值的文件数量、单条目解压大小及总解压大小。所有 OFD 内部路径都通过同一个解析器相对于声明文件定位，移除当前的 `Annots` 等硬编码。

### 5.2 解析与领域模型

解析模块把标准 XML 映射为内部原始结构，再由加载器完成引用解析和基本校验。领域模型以文档、页面和资源 ID 为边界，不允许渲染器直接操作 ZIP archive。

Document 在打开时只加载入口、文档根、公共元数据和页面索引。页面 XML、字体、图片、注释及签名按需加载，并在 Document 内部缓存。这样能支持大文档，同时让外部 `Document` 保持只读和线程安全演进空间。

### 5.3 显示列表

页面解析结果先转换为与 Cairo 无关的显示列表：

```text
Save
ConcatTransform
ClipPath
SetPaint
DrawPath
DrawGlyphRun
DrawImage
Restore
```

模板页、页面层和嵌套 PageBlock 都被展开为有序命令。显示列表统一处理绘制状态栈、边界、CTM 和裁剪；Cairo 只是第一个消费该列表的后端。文字提取保留原始文字、字形位置和阅读顺序信息，不从最终位图反推。

### 5.4 资源系统

资源解析以文档资源和公共资源为两个命名空间，通过资源 ID 查询。字体资源支持嵌入字体优先、系统字体回退和缺失字体报告；图像首版支持 PNG 与 JPEG，其余格式返回具体的不支持诊断。解码后的字体和图像按资源 ID 缓存。

### 5.5 注释与签名

v0.2 解析注释列表、边界、类型和外观，并把印章外观加入显示列表。签名 API 返回签名者、时间、覆盖范围、算法标识及验证状态。

验证状态固定为 `NotChecked`、`Valid`、`Invalid` 和 `Unsupported`。在没有完成密码算法和证书链校验时绝不能把“成功解析签名文件”等同于“签名有效”。首版允许只实现结构解析与 `NotChecked`/`Unsupported`，但接口形态必须稳定。

## 6. 错误与兼容策略

统一错误枚举至少区分：I/O、ZIP 容器、XML、无效结构、缺失资源、页面越界、密码需求、不支持特性、资源限制、渲染和内部错误。错误包含 OFD 内部文件路径及必要上下文，但不能泄露无关主机路径或造成二次 panic。

严格模式遇到违反标准或不支持且影响正确性的内容立即失败。宽容模式可以跳过不影响主体阅读的未知扩展，但必须收集 warnings。默认使用宽容模式，并始终执行安全限制。

## 7. Qt/QML Demo

Demo 是库能力的验收应用，而不是第二套 OFD 实现。功能范围包括：

- 文件打开与最近文件。
- 多页连续滚动与当前页指示。
- 放大、缩小、100%、适合宽度和适合页面。
- 页码输入与上一页/下一页。
- 文本搜索、结果计数、上一项/下一项和页面高亮。
- 文档属性，包括元数据、页面数、权限、签名及 warnings。
- 加载失败、不支持内容和损坏文件的明确错误页。

页面采用可见区域优先的异步渲染和有限大小缓存。缩放变化时取消过期任务；后台任务只产生像素缓冲，QImage/QML 对象在 GUI 线程创建或更新。

## 8. 测试与质量门槛

### 8.1 测试层次

- 单元测试：数值类型、路径、颜色、矩阵、路径指令和错误映射。
- 解析测试：每种 XML 节点、可选字段、命名空间和错误引用。
- 语料测试：多页、模板、字体、图片、注释、签名及不同生产商文件。
- 像素回归：固定字体环境、固定 DPI，比较页面基准图并给出差异图。
- 安全测试：路径逃逸、ZIP bomb 限制、畸形 XML、循环引用和超大数值。
- FFI 测试：纯 C 编译、打开/查询/渲染、错误读取和生命周期。
- GUI 测试：打开文件、翻页、缩放、搜索和错误提示的最小自动化覆盖。
- 模糊测试：容器入口、XML 解析、数值及路径解析器。

### 8.2 v0.2 发布门槛

- 核心库在不安装 Qt/GTK 的干净环境中构建和测试通过。
- 所有公开输入路径对畸形文档均返回错误，不发生 panic。
- 测试语料中的受支持页面达到定义好的像素差异阈值。
- Rust 示例与纯 C 示例都能打开、查询和渲染同一份文档。
- Demo 可完成打开、多页浏览、缩放、跳页、搜索和属性查看。
- CI 包含 format、clippy、核心测试、FFI 测试、渲染回归和 Qt Demo 构建。
- 公共 Rust API、C 头文件、支持矩阵和已知限制均有文档。

## 9. 分阶段交付

### 阶段 0：基线与构建解耦

建立 Cargo workspace，分离 GUI 依赖，保存当前样例渲染基准，并使核心库独立通过 CI。

### 阶段 1：Document/Page API 与安全容器

完成稳定的文档、页面、元数据、错误、加载选项和安全 ZIP 访问接口；支持按需加载多页文档。

### 阶段 2：显示列表与基础渲染

实现绘制状态、坐标、CTM、裁剪、路径、基础颜色、模板及页面层，并迁移现有 Cairo 输出。

### 阶段 3：文字、字体和图像

实现字形定位、嵌入字体与回退、文字区域、PNG/JPEG 图像、资源缓存和按页像素输出。

### 阶段 4：文本查询、注释和签名信息

提供文字提取、搜索命中框、注释外观、签名结构与明确验证状态。

### 阶段 5：C ABI v1

实现不透明句柄、错误对象、Cairo 渲染入口、头文件生成/校验和纯 C 集成测试，并冻结 v0.2 所需 ABI。

### 阶段 6：Qt/QML Demo

通过 C ABI 实现阅读器功能、异步页面渲染、缓存、搜索高亮和属性面板。

### 阶段 7：兼容性与发布

扩充多厂商语料、像素回归、安全测试和性能基准，补齐文档、示例与安装产物，发布 v0.2。

## 10. 后续版本边界

v0.3 以后再按真实需求加入更多图像格式、复杂颜色空间与渐变、链接和动作、附件提取、权限策略、密码与解密、完整签名验证、打印优化和 GLib/GObject 封装。编辑、表单填写和签章创建应作为独立子项目设计，不与只读阅读库混在同一实施计划中。
