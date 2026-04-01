#!/usr/bin/env python3

from fontTools.ttLib import TTFont
from fontTools.pens.t2CharStringPen import T2CharStringPen
import os
import urllib.request

GONOTO_URL = "https://github.com/satbyy/go-noto-universal/releases/download/v7.0/GoNotoKurrent-Regular.ttf"
NOTO_EMOJI_URL = "https://github.com/googlefonts/noto-emoji/raw/main/fonts/NotoColorEmoji.ttf"

def download_font(url, output_path):
    print(f"Downloading from {url}")
    urllib.request.urlretrieve(url, output_path)
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
        'GPOS', 'kern', 'BASE', 'JSTF',
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
    font.flavor = None
    font.save(output_path)
    font.close()

    print("Done!")

if __name__ == "__main__":
    script_dir = os.path.dirname(os.path.abspath(__file__))
    assets_dir = os.path.join(script_dir, "..", "assets")

    print("=== Creating Noto-Phantom.ttf (Text) ===")
    temp_text = os.path.join(script_dir, "GoNotoKurrent-Regular.ttf")
    output_text = os.path.join(assets_dir, "Noto-Phantom.ttf")
    download_font(GONOTO_URL, temp_text)
    create_phantom_font(temp_text, output_text)
    os.remove(temp_text)
    print(f"Cleaned up: {temp_text}\n")

    print("=== Creating Noto-Phantom-Emoji.ttf (Emoji) ===")
    temp_emoji = os.path.join(script_dir, "NotoColorEmoji.ttf")
    output_emoji = os.path.join(assets_dir, "Noto-Phantom-Emoji.ttf")
    download_font(NOTO_EMOJI_URL, temp_emoji)
    create_phantom_font(temp_emoji, output_emoji)
    os.remove(temp_emoji)
    print(f"Cleaned up: {temp_emoji}\n")

    print("=== All phantom fonts created ===")
