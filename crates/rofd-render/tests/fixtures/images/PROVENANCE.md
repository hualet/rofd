# Phase 3 image fixtures

`asymmetric-rgba.png` and `asymmetric-rgb.jpg` are programmatically generated
3 by 2 pixel test images. They contain no third-party artwork or other licensed
material.

They were generated with ImageMagick 7.1.2-27 using:

```text
convert -size 3x2 xc:none -fill '#ff0000ff' -draw 'point 0,0' -fill '#00ff0080' -draw 'point 1,0' -fill '#0000ffff' -draw 'point 2,0' -fill '#ffff00ff' -draw 'point 0,1' -fill '#ff00ffff' -draw 'point 1,1' -fill '#00ffffff' -draw 'point 2,1' -strip -define png:exclude-chunk=date,time asymmetric-rgba.png
convert -size 3x2 xc:white -fill '#e02020' -draw 'point 0,0' -fill '#20e020' -draw 'point 1,0' -fill '#2020e0' -draw 'point 2,0' -fill '#e0e020' -draw 'point 0,1' -fill '#e020e0' -draw 'point 1,1' -fill '#20e0e0' -draw 'point 2,1' -sampling-factor 1x1 -quality 95 -strip asymmetric-rgb.jpg
```

SHA-256 checksums:

```text
8d77c7c7fd4a9b6b6aa677f2b5bf0af0a3da03bed33be3cfbcc5d3a50f89eb35  asymmetric-rgb.jpg
f1eeb4a7c6afe425ad003957df0c8fe11d6c111c6012f2e2f810fb16ceb9009c  asymmetric-rgba.png
```
