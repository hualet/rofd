# Test fixtures

`gbk-entry-name.ofd` is a programmatically generated, minimal OFD package. It
contains no third-party artwork or other licensed material.

All entries have ASCII names except `Doc_0/资源/图像_10.bmp`, whose ZIP entry
name is encoded as GBK bytes without the UTF-8 flag (language encoding flag
0x800 unset), mimicking packages produced by generators that write
non-UTF-8 entry names. The package XML references the same path as proper
UTF-8 text, so the two never match.

Generated with Python 3 `zipfile`, overriding
`ZipInfo._encodeFilenameFlags` to emit `filename.encode("gbk")` with flag
bits 0 for the non-ASCII entry.
