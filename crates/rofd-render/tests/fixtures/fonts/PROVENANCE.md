# Phase 3 font fixtures

`phase3-subset.ttf` and `phase3-latin-subset.ttf` are test-only modified
subsets of Noto Sans CJK SC Regular, version 2.004, face 2 of Debian's
`/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc` from package
`fonts-noto-cjk`. Upstream source: <https://github.com/notofonts/noto-cjk>.

Upstream copyright: Copyright 2010-2012 Google Corporation. The font and
these modified subsets are distributed under the SIL Open Font License 1.1;
see `OFL-1.1.txt`. No Reserved Font Names were declared in the Debian source
copyright file.

The fixtures were generated with fonttools `pyftsubset` using:

```text
python3 -m fontTools.subset /usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc --font-number=2 --unicodes=U+0020,U+0041-0043,U+005A,U+25A1,U+4E2D,U+6587,U+FFFD --layout-features='*' --name-IDs='*' --name-legacy --name-languages='*' --glyph-names --symbol-cmap --legacy-cmap --notdef-glyph --notdef-outline --recommended-glyphs --output-file=crates/rofd-render/tests/fixtures/fonts/phase3-subset.ttf
python3 -m fontTools.subset /usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc --font-number=2 --unicodes=U+0020,U+0041-0043,U+005A,U+25A1 --layout-features='*' --name-IDs='*' --name-legacy --name-languages='*' --glyph-names --symbol-cmap --legacy-cmap --notdef-glyph --notdef-outline --recommended-glyphs --output-file=crates/rofd-render/tests/fixtures/fonts/phase3-latin-subset.ttf
```

The full subset intentionally contains controlled Latin (`A`-`C`, `Z`),
CJK (`中`, `文`), and visible replacement-box (`□`) glyphs. The
Latin subset omits CJK so mixed per-character fallback is deterministic.
