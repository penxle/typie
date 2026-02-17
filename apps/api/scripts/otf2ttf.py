"""OTF(CFF) → TTF(glyf) 변환. stdin에서 OTF를 읽고 stdout에 TTF를 쓴다."""
import sys
from io import BytesIO

from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont, newTable


MAX_ERR = 1.0


def otf_to_ttf(otf_data: bytes) -> bytes:
    font = TTFont(BytesIO(otf_data))

    assert "CFF " in font, "Input is not a CFF-based font"

    glyphOrder = font.getGlyphOrder()
    glyphSet = font.getGlyphSet()

    # Convert cubic bezier curves to quadratic
    quadGlyphs = {}
    for glyphName in glyphOrder:
        glyph = glyphSet[glyphName]
        ttPen = TTGlyphPen(glyphSet)
        cu2quPen = Cu2QuPen(ttPen, MAX_ERR, reverse_direction=True)
        glyph.draw(cu2quPen)
        quadGlyphs[glyphName] = ttPen.glyph()

    # Replace CFF with glyf table
    glyf = newTable("glyf")
    glyf.glyphOrder = glyphOrder
    glyf.glyphs = quadGlyphs
    font["glyf"] = glyf

    # Add required loca table
    font["loca"] = newTable("loca")

    # Remove CFF-specific tables
    del font["CFF "]
    if "CFF2" in font:
        del font["CFF2"]

    # Set sfVersion to TrueType
    font.sfntVersion = "\x00\x01\x00\x00"

    out = BytesIO()
    font.save(out)
    return out.getvalue()


if __name__ == "__main__":
    data = sys.stdin.buffer.read()
    result = otf_to_ttf(data)
    sys.stdout.buffer.write(result)
