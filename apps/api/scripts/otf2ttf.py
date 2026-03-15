#!/usr/bin/env python3
"""Convert OTF (CFF outline) fonts to TTF (glyf outline) fonts.

Usage:
    python3 otf2ttf.py <input.otf> [output.ttf]
    python3 otf2ttf.py NotoSansKR-Regular.otf
    python3 otf2ttf.py NotoSansKR-Regular.otf NotoSansKR-Regular.ttf

If output path is omitted, replaces .otf with .ttf in the input filename.

Requirements:
    pip install fonttools cu2qu skia-pathops
"""

import sys
import os
from fontTools.ttLib import TTFont
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib.tables._g_l_y_f import table__g_l_y_f, Glyph as TTGlyphObj
from fontTools.ttLib.tables._l_o_c_a import table__l_o_c_a


def convert_otf_to_ttf(input_path: str, output_path: str) -> None:
    font = TTFont(input_path)

    if "CFF " not in font:
        print(f"  {input_path} is not a CFF font, skipping.")
        font.close()
        return

    cff = font["CFF "]
    top_dict = cff.cff.topDictIndex[0]
    char_strings = top_dict.CharStrings
    glyph_order = font.getGlyphOrder()
    max_err = font["head"].unitsPerEm / 1000

    # Convert CFF cubic curves to TrueType quadratic curves
    tt_glyphs = {}
    for gname in glyph_order:
        ttpen = TTGlyphPen(None)
        cu2qupen = Cu2QuPen(ttpen, max_err, reverse_direction=True)
        try:
            char_strings[gname].draw(cu2qupen)
            tt_glyphs[gname] = ttpen.glyph()
        except Exception:
            tt_glyphs[gname] = TTGlyphObj()

    # Remove CFF tables, add TrueType tables
    del font["CFF "]
    if "VORG" in font:
        del font["VORG"]

    glyf_table = table__g_l_y_f()
    glyf_table.glyphs = tt_glyphs
    glyf_table.glyphOrder = glyph_order
    font["glyf"] = glyf_table
    font["loca"] = table__l_o_c_a()

    font.sfntVersion = "\x00\x01\x00\x00"
    font["head"].indexToLocFormat = 1

    # Ensure all glyphs have bounding box attributes (needed for maxp.recalc)
    glyf_obj = font["glyf"]
    for gname in glyph_order:
        g = glyf_obj[gname]
        if not hasattr(g, "xMin"):
            g.xMin = g.yMin = g.xMax = g.yMax = 0

    # Fix maxp for TrueType
    maxp = font["maxp"]
    maxp.tableVersion = 0x00010000
    maxp.maxZones = 1
    maxp.maxTwilightPoints = 0
    maxp.maxStorage = 0
    maxp.maxFunctionDefs = 0
    maxp.maxInstructionDefs = 0
    maxp.maxStackElements = 0
    maxp.maxSizeOfInstructions = 0
    maxp.recalc(font)

    font.save(output_path)
    font.close()

    # Verify
    check = TTFont(output_path)
    size = os.path.getsize(output_path)
    cps = len(check.getBestCmap())
    weight = check["OS/2"].usWeightClass
    check.close()
    print(f"  {output_path}: {cps} codepoints, weight={weight}, {size / 1024 / 1024:.1f}MB")


def main() -> None:
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <input.otf> [output.ttf]")
        sys.exit(1)

    input_path = sys.argv[1]
    if len(sys.argv) >= 3:
        output_path = sys.argv[2]
    else:
        base, _ = os.path.splitext(input_path)
        output_path = base + ".ttf"

    print(f"Converting {input_path} -> {output_path}")
    convert_otf_to_ttf(input_path, output_path)
    print("Done.")


if __name__ == "__main__":
    main()
