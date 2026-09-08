#!/usr/bin/env python3
"""Generate a deterministic mixed-size OFD for reader interaction QA."""
import sys
import zipfile
from xml.sax.saxutils import escape

NS = 'xmlns:ofd="http://www.ofdspec.org/2016"'
SIZES = [(210, 297), (297, 210), (180, 260), (210, 297), (210, 297)]

def make_fixture(path):
    entries = {
        "OFD.xml": f'<ofd:OFD {NS} DocType="OFD" Version="1.0"><ofd:DocBody><ofd:DocInfo><ofd:DocID>reader-qa</ofd:DocID><ofd:Title>阅读体验测试</ofd:Title></ofd:DocInfo><ofd:DocRoot>Doc/Document.xml</ofd:DocRoot></ofd:DocBody></ofd:OFD>',
        "Doc/Res.xml": f'<ofd:Res {NS}><ofd:Fonts><ofd:Font ID="1" FontName="Noto Sans CJK SC" FamilyName="Noto Sans CJK SC"/></ofd:Fonts></ofd:Res>',
    }
    refs = ''.join(f'<ofd:Page ID="{i + 100}" BaseLoc="Page{i}.xml"/>' for i in range(len(SIZES)))
    entries['Doc/Document.xml'] = f'<ofd:Document {NS}><ofd:CommonData><ofd:MaxUnitID>999</ofd:MaxUnitID><ofd:PublicRes>Res.xml</ofd:PublicRes><ofd:PageArea><ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox></ofd:PageArea></ofd:CommonData><ofd:Pages>{refs}</ofd:Pages></ofd:Document>'
    for i, (w, h) in enumerate(SIZES):
        objects = []
        def text(object_id, x, y, size, content, color='45 57 75'):
            objects.append(f'<ofd:TextObject ID="{object_id}" Boundary="{x} {y} {w - x - 20} {size * 1.8}" Font="1" Size="{size}"><ofd:FillColor Value="{color}"/><ofd:TextCode X="0" Y="{size * 1.1}">{escape(content)}</ofd:TextCode></ofd:TextObject>')
        text(10, 22, 18, 3, 'ROFD  /  READER NOTES', '54 117 223')
        text(11, 22, 35, 9, ['阅读，让信息更清晰', '横向页面，自然展开', '轻松找到每一处文字', '专注内容，从容阅读', '阅读体验，始终如一'][i])
        text(12, 22, 56, 3.5, '阅读体验测试 · 连续滚动 / 页面导航 / 全文搜索', '110 123 140')
        for n, line in enumerate([
            '在连续页面之间流畅浏览，保持思路连贯。',
            '调整缩放比例，让文字与图表以合适的尺寸呈现。',
            '使用侧栏缩略图，快速找到需要阅读的页面。',
            '输入关键词，即可搜索全文并跳转到匹配的位置。',
            '这份文档包含不同尺寸的页面，用于验证阅读体验。',
            'English search: Reader notes / RUST / Unicode.',
        ]):
            text(20 + n, 22, 85 + n * 12, 4, line)
        text(40, 22, h - 24, 3, f'阅读体验测试                                    {i + 1:02} / 05', '130 142 158')
        entries[f'Doc/Page{i}.xml'] = f'<ofd:Page {NS}><ofd:Area><ofd:PhysicalBox>0 0 {w} {h}</ofd:PhysicalBox></ofd:Area><ofd:Content><ofd:Layer ID="2">{"".join(objects)}</ofd:Layer></ofd:Content></ofd:Page>'
    with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as archive:
        for name, content in entries.items():
            info = zipfile.ZipInfo(name, (2026, 9, 7, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(info, content.encode('utf-8'))

if __name__ == '__main__':
    make_fixture(sys.argv[1])
