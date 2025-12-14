#!/usr/bin/env python3

from fontTools.ttLib import TTFont
from fontTools.pens.t2CharStringPen import T2CharStringPen
import os
import sys
import urllib.request

GONOTO_URL = "https://github.com/satbyy/go-noto-universal/releases/download/v7.0/GoNotoKurrent-Regular.ttf"

def download_gonoto(output_path):
    print(f"Downloading GoNotoKurrent from {GONOTO_URL}")
    urllib.request.urlretrieve(GONOTO_URL, output_path)
    print(f"Downloaded to {output_path}")

def create_phantom_font(input_path, output_path):
    print(f"Loading font: {input_path}")
    font = TTFont(input_path)

    if 'glyf' in font:
        print("Processing TrueType font (glyf table)")
        glyf_table = font['glyf']
        glyph_count = len(glyf_table.keys())

        for idx, glyph_name in enumerate(glyf_table.keys()):
            glyph = glyf_table[glyph_name]
            glyph.numberOfContours = 0
            glyph.coordinates = []
            glyph.flags = []
            glyph.endPtsOfContours = []

            if (idx + 1) % 1000 == 0:
                print(f"  Processed {idx + 1}/{glyph_count} glyphs")

        print(f"  Total glyphs processed: {glyph_count}")

    elif 'CFF ' in font:
        print("Processing PostScript/CFF font (CFF table)")
        cff = font['CFF ']
        top_dict = cff.cff.topDictIndex[0]
        char_strings = top_dict.CharStrings
        glyph_count = len(char_strings.keys())

        for idx, glyph_name in enumerate(char_strings.keys()):
            pen = T2CharStringPen(width=0, glyphSet=None)
            char_strings[glyph_name] = pen.getCharString()

            if (idx + 1) % 1000 == 0:
                print(f"  Processed {idx + 1}/{glyph_count} glyphs")

        print(f"  Total glyphs processed: {glyph_count}")

    tables_to_remove = [
        'GPOS', 'GSUB', 'kern', 'GDEF', 'BASE', 'JSTF',
        'DSIG', 'SVG ', 'sbix', 'CBDT', 'CBLC', 'COLR', 'CPAL',
        'VORG', 'VDMX', 'hdmx', 'LTSH', 'prep', 'fpgm', 'cvt ',
        'gasp'
    ]

    removed = []
    for table in tables_to_remove:
        if table in font:
            del font[table]
            removed.append(table)

    if removed:
        print(f"Removed tables: {', '.join(removed)}")

    print(f"Saving phantom font: {output_path}")
    font.save(output_path)
    font.close()

    print("Done!")

if __name__ == "__main__":
    script_dir = os.path.dirname(os.path.abspath(__file__))
    temp_font = os.path.join(script_dir, "GoNotoKurrent-Regular.ttf")
    output_file = os.path.join(script_dir, "Noto-Phantom.ttf")

    download_gonoto(temp_font)
    create_phantom_font(temp_font, output_file)
    os.remove(temp_font)
    print(f"Cleaned up temporary file: {temp_font}")
