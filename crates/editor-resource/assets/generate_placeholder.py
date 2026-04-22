#!/usr/bin/env python3
"""Generate a minimum-valid TTF used as the typie editor placeholder font.

Properties:
- units_per_em = 1000
- one glyph: .notdef with an empty outline (0 contours)
- cmap contains no mappings (every codepoint resolves to glyph id 0)
- hmtx.advance for .notdef = 500 (0.5 em)
- ascent = 800, descent = -200, line_gap = 0

Run: `python3 generate_placeholder.py` from this directory.
Writes `placeholder.ttf` next to the script.
"""
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

FAMILY = "__typie_placeholder__"
UPEM = 1000

fb = FontBuilder(UPEM, isTTF=True)

glyph_order = [".notdef"]
fb.setupGlyphOrder(glyph_order)

fb.setupCharacterMap({})

pen = TTGlyphPen(None)
glyphs = {".notdef": pen.glyph()}
fb.setupGlyf(glyphs)

fb.setupHorizontalMetrics({".notdef": (500, 0)})

fb.setupHorizontalHeader(ascent=800, descent=-200, lineGap=0)

fb.setupOS2(
    sTypoAscender=800,
    sTypoDescender=-200,
    sTypoLineGap=0,
    usWinAscent=800,
    usWinDescent=200,
)

fb.setupNameTable({"familyName": FAMILY, "styleName": "Regular"})
fb.setupPost()

fb.save("placeholder.ttf")
print("wrote placeholder.ttf")
