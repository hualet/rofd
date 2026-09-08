# Fixture 来源说明（PROVENANCE）

本目录（`fixtures/`）下的所有 `.ofd` 文件均复制自 [ofdrw](https://github.com/ofdrw/ofdrw)
项目各模块的 `src/test/resources/` 测试资源，未做任何修改。

- 来源仓库：https://github.com/ofdrw/ofdrw
- 来源 commit：`7459e35082170061efa6b399a6518dbc219f08ac`（2026-08-04）
- 许可：Apache License 2.0（见 ofdrw 仓库根目录 `LICENSE`）

`references/` 下的参考 PNG 由 ofdrw 的 `ImageMaker`（96 DPI、`drawBoundary=false`）
渲染上述 fixture 生成，生成工具见 `tools/RenderReferences.java` 与
`tools/render-references.sh`，逐文件渲染结果记录在 `references/manifest.json`。

## 复制的文件

### `fixtures/reader/`（来自 `ofdrw-reader/src/test/resources/`）

| 文件 | 大小（字节） | ofdrw 中的用途 |
|---|---:|---|
| helloworld.ofd | 1,652 | OFDReaderTest / ContentExtractorTest / ResourceLocatorTest |
| helloworld_with_pageblock.ofd | 1,974 | ContentExtractorTest.extractAllPageBlock |
| chineseDir.ofd | 2,097 | OFDReaderTest.testChineseDirName |
| multiKeywordInTextCode.ofd | 1,928 | KeywordExtractorTest.testKeyword |
| keyword.ofd | 114,993 | KeywordExtractorTest |
| keyword2.ofd | 2,991 | KeywordExtractorTest.testKeyword2 |
| path_unstd.ofd | 8,200 | IssueCase.github_293（回归用例） |
| AddAttachment.ofd | 38,670 | OFDReaderTest.getAttachment |
| SESV4SignDoc.ofd | 35,814 | OFDReaderTest.getStampAnnots / getPageSize |
| 发票示例.ofd | 46,091 | 发票样例 |

### `fixtures/converter/`（来自 `ofdrw-converter/src/test/resources/`）

1.ofd (16,048)、20240531141733.ofd (46,091)、999.ofd (29,752)、不规范资源路径.ofd (627,373)、
发票监制章-数科.ofd (19,424)、发票示例.ofd (13,584，与 reader 版内容不同，两份都保留)、
透明度文字.ofd (3,763)、文字横向-数科.ofd (49,003)、ano.ofd (718,318)、containsJPEG.ofd (898,382)、
draw_param_ref.ofd (33,402)、helloworld.ofd (1,651)、h.ofd (73,148)、intro-数科.ofd (7,490,390)、
n.ofd (39,835)、pattern类型.ofd (490,897)、signout.ofd (158,072)、SignScaleError.ofd (17,438)、
testImageNotFound.ofd (173,623)、testImageOverridePage.ofd (255,199)、testPathClip.ofd (784,902)、
testPathFillOpacity.ofd (65,178)、V4RideRight.ofd (39,170)、y.ofd (2,209,020)、z.ofd (39,704)、
zsbk.ofd (1,557,477)

这些文件服务于 `OFD2IMGTest`、`OFD2PDFTest`、`OFD2SVGTest`、`ImageExporterTest` 等渲染/导出测试。

### `fixtures/layout/`（来自 `ofdrw-layout/src/test/resources/`）

- no_page_container.ofd (7,707)：页面容器结构非标准，WatermarkTest 使用
- 拿来主义_page6.ofd (5,226)

### `fixtures/sign/`（来自 `ofdrw-sign/src/test/resources/`）

- namespace_no_std.ofd (1,943)：XML 命名空间非标准，OFDSignerTest 使用

## 明确排除的文件及原因

| 文件 | 原因 |
|---|---|
| ofdrw-layout/helloworld.ofd（19 MB） | writer 回写产物，体积过大且无对应断言用例 |
| NotoSerifCJKsc-Regular.otf / NotoSerifCJKsc-Medium.otf（约 23.6 MB） | ofdrw 布局测试用字体，rofd 渲染使用文档内嵌字体或系统字体 |
| ofdrw-converter 的 font_10.ttf、font_13132_0_edit.ttf、type1_cff.otf、img.jpg | ofdrw 布局/导出测试素材，rofd 只读解析用不到 |
| ofdrw-layout 的 keyword.ofd、AddAttachment.ofd 等重复文件 | 与 reader 版本内容相同，已去重 |
| ofdrw-crypto / ofdrw-archive / ofdrw-tool / ofdrw-sign 的其余文件 | 仅服务于签名、加密、合并等 rofd 不支持的功能流程 |
| ofdrw-reader 的 DOC_0.zip、namespace_case.xml | 仅服务于 ZipUtil / NameSpaceCleaner 等 ofdrw 内部工具测试 |
