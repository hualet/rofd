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

`phase3-subsets.ttc` is a deterministic two-face collection assembled only
from those two subsets. Face 0 is the Latin subset and face 1 is the full
Latin+CJK subset. It was generated with fonttools using:

```text
python3 - <<'PY'
from fontTools.ttLib import TTCollection, TTFont
base = 'crates/rofd-render/tests/fixtures/fonts/'
collection = TTCollection()
collection.fonts = [
    TTFont(base + 'phase3-latin-subset.ttf'),
    TTFont(base + 'phase3-subset.ttf'),
]
collection.save(base + 'phase3-subsets.ttc')
PY
```

SHA-256 checksums:

```text
85bb7ce3de6b714de6ebb4b386e2fdc7d9f2adb9085be1cd079d87d0c2ed375a  phase3-subset.ttf
029cf0083eb69e438f3545e2b67e8d0d60e03ae27f9b959741de00d5fbb9ddbe  phase3-latin-subset.ttf
9323f7c7c65aa5365f06511105a27cea6aaafc44df3ab3f90106804fef60ccd7  phase3-subsets.ttc
```
