# ofdrw-compat：从 ofdrw 迁移的兼容性测试

本 crate 的测试用例与 fixture 迁移自 [ofdrw](https://github.com/ofdrw/ofdrw)
（Java 实现的 OFD 读写库，Apache License 2.0），来源 commit 为
`7459e35082170061efa6b399a6518dbc219f08ac`（2026-08-04）。fixture 与参考图的
逐文件来源见 [PROVENANCE.md](PROVENANCE.md)。

迁移目的：用 ofdrw 积累的真实世界测试文件和断言扩大 rofd 的测试范围——
包括真实 `.ofd` 文件的解析断言、非标准包结构的边界用例，以及与 ofdrw
渲染结果的自动像素对比。

## 运行

```bash
cargo test -p ofdrw-compat
```

本 crate 不在 workspace 的 `default-members` 中，需要显式 `-p ofdrw-compat`。
渲染对比依赖系统安装的 Noto CJK 字体（与 `rofd-render` 的既有测试一致）。

## 目录

- `fixtures/` — 从 ofdrw 各模块 `src/test/resources/` 复制的真实 `.ofd` 文件。
- `references/` — 由 ofdrw 的 `ImageMaker`（96 DPI）渲染 fixture 得到的参考
  PNG，`manifest.json` 记录每个文件的渲染结果。重新生成方法见下文。
- `tools/` — 参考图生成器（Java 源码 + 复现脚本），不属于 Rust 构建。
- `tests/reader_parse.rs` — 解析与文本提取断言（移植自 ofdrw-reader）。
- `tests/package_edge.rs` — 非标准包结构冒烟 + 全部 fixture 的加载回归。
- `tests/render_compare.rs` — rofd 渲染结果与 ofdrw 参考 PNG 的容差对比。
- `tests/support/mod.rs` — 打开/渲染/文本提取/像素 diff 辅助，以及
  `KNOWN_LOAD_FAILURES`（已知解析差异）和逐 fixture 的渲染阈值表。

## 测试映射表

### 已移植（ofdrw-reader）

| ofdrw 测试 | 本 crate 测试 | 说明 |
|---|---|---|
| OFDReaderTest.getOFDDir | `ofd_reader_get_ofd_dir_doc_id` | DocID 精确断言 |
| OFDReaderTest.getPage | `ofd_reader_get_page_has_single_layer` | 页 layer 数 |
| OFDReaderTest.getPageSize | `ofd_reader_ses_v4_page_size` | 210×297 mm |
| ContentExtractorTest.getPageContent / extractAll / traverse | `content_extractor_helloworld_text` | 整页文本精确断言 |
| ContentExtractorTest.extractAllPageBlock | `content_extractor_pageblock_text` | 含 pageblock 的文本 |
| KeywordExtractorTest.testKeyword | `keyword_extractor_multi_keyword_occurrences` | ofdrw 断言 7 个坐标，降级为出现次数（rofd 无关键字坐标 API） |
| KeywordExtractorTest.getKeyWordPositionList | `keyword_extractor_keyword_contains`（ignored） | 降级为包含断言；fixture 触发已知解析差异 |
| KeywordExtractorTest.testKeyword2 | `keyword_extractor_keyword2_contains` | 降级为包含断言 |
| IssueCase.github_293 | `issue_case_github_293_unstandard_page_dirs`（ignored） | fixture 触发已知解析差异 |
| —（ofdrw 渲染测试均为肉眼检查） | `rendered_pages_match_ofdrw_references` | 用 ofdrw 渲染结果作 golden reference 自动对比 |
| —（边界 fixture 散落在多个模块） | `namespace_no_std_loads` / `nalazhuyi_page6_loads` / `no_page_container_loads`（ignored）/ `non_standard_resource_paths_load`（ignored） | 非标准包结构冒烟 |
| — | `all_migrated_fixtures_open_and_load_pages` | 全部 39 个 fixture 的打开+逐页加载回归 |

### 未移植及原因

| ofdrw 测试 | 原因 |
|---|---|
| OFDReaderTest.testChineseDirName / getAttachment | rofd-core 暂无附件（Attachments）API |
| OFDReaderTest.getStampAnnots | rofd-core 暂无批注（Annotations）API |
| OFDReaderTest.lowLevelOp | 写操作，rofd 是只读库 |
| OFDReaderTest.close / getWorkDir | ofdrw 解压到工作目录的实现细节，rofd 不解压 |
| OFDReaderStreamTest | 流式打开；`Document::from_bytes` 已覆盖等价路径 |
| ResourceLocatorTest | ofdrw 内部虚拟路径 API，无对应物 |
| ZipTest / ZipUtilTest | zip-slip 防护针对"解压到磁盘"，rofd 全部内存读取，不适用 |
| NameSpaceCleanerTest / NameSpaceModifierTest | ofdrw 内部 XML 工具 |
| ofdrw-core 全部 XML round-trip 测试（约 87 个） | 测的是 ofdrw 的元素序列化框架，rofd-core 解析器架构不同，已由 `crates/rofd-core/tests/` 覆盖 |
| ofdrw-layout / ofdrw-sign / ofdrw-gm / ofdrw-crypto | 文档写入、签名、国密，rofd 不支持 |
| ofdrw-converter 的 PDF/SVG/HTML/文本导出 | rofd-render 只有位图渲染 |

## 已知差异（KNOWN_LOAD_FAILURES）

以下 fixture 目前无法被 rofd-core 完整加载，记录在
`tests/support/mod.rs` 的 `KNOWN_LOAD_FAILURES` 表中；全量冒烟测试会校验
"表外文件不得失败、表内文件一旦能打开就必须删表项"：

- 不支持的图像格式 JB2 / GBIG2：layout/no_page_container.ofd、converter/1.ofd

早期迁移时记录的解析差异（前导斜杠包路径、缺 PageArea、零/负 Boundary、
DeltaX/Y 个数与符号、首个 TextCode 省略原点、对象缺 ID、重复 Fonts 元素、
非连续 TemplatePage 声明、`S` 路径操作符、空格/井号分隔的颜色通道
`#ee #20 #25`、无 Value 的 FillColor/StrokeColor、PathObject 缺 Boundary
等）已逐项修复；ofdrw 宽容而 rofd 严格的写法以
"lenient 容忍、strict 报错"的方式支持。

## 已知渲染差异

能打开但渲染层面与 ofdrw 有差距的 fixture，记录在 `support/mod.rs` 的
`KNOWN_RENDER_FAILURES` / `UNUSABLE_REFERENCES` / 阈值表中：

- converter/y.ofd 第 2-3 页：正文使用未内嵌字体的显式 glyph ID，rofd 把
  glyph ID 应用到回退字体而 ofdrw 用它自己的默认字体，字形表不同导致
  正文文字形状不同（不匹配约 16%，阈值放宽至 18% 并注明原因）
- converter/pattern类型.ofd：**ofdrw 自己渲染成全黑页**（其 pattern 填充 bug），
  而 rofd 渲染出了可见的标题文字；参考图不可用，排除出对比

渲染调试工具：`ROFD_DUMP="converter/y.ofd:1,2" cargo test -p ofdrw-compat --test dump -- --ignored`，
输出到 `target/` 下，不影响已提交的参考图。

## 重新生成渲染参考图

需要 JDK 与 Maven（一次性步骤，生成结果已提交到仓库）：

```bash
tests/ofdrw-compat/tools/render-references.sh [/path/to/ofdrw 检出]
```

脚本会构建 ofdrw（`-Dgpg.skip=true`）、编译 `tools/RenderReferences.java`、
把 `fixtures/` 中每个文件按 96 DPI 渲染到 `references/<模块>/<文件名>.ofd/<页码>.png`，
并写 `manifest.json`。每个文件最多渲染前 5 页以控制仓库体积。
**生成后请人工抽查 PNG 再提交**——参考图即测试基准。

## 渲染对比的判定

逐像素比较 RGB 通道，通道差 ≤ 32 视为一致（吸收 Java2D 与 cairo 的抗锯齿
差异）；整页不匹配像素比例须低于该 fixture 的阈值（默认 10%，可在
`support/mod.rs` 的阈值表中逐文件调整）。页面像素尺寸允许 ±2px 的舍入差异，
超出则直接判失败。
