#!/usr/bin/env python3
"""Download Noto Sans variable fonts from google/fonts and instantiate static TTFs.

Usage:
    python3 instantiate-variable-fonts.py <output-dir>

Downloads variable fonts from the google/fonts GitHub repo, then uses
fontTools.varLib.instancer to pin axes and produce static TTF instances.

Requirements:
    pip install fonttools
"""

import os
import sys
import tempfile
import urllib.request

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

GITHUB_RAW = "https://raw.githubusercontent.com/google/fonts/main"

VARIABLE_FONTS = [
    ("ofl/notosans/NotoSans%5Bwdth%2Cwght%5D.ttf", [
        ("NotoSans-Regular.ttf", {"wght": 400, "wdth": 100}, "Noto Sans", 400),
        ("NotoSans-Bold.ttf", {"wght": 700, "wdth": 100}, "Noto Sans", 700),
    ]),
    ("ofl/notosanskr/NotoSansKR%5Bwght%5D.ttf", [
        ("NotoSansKR-Regular.ttf", {"wght": 400}, "Noto Sans KR", 400),
        ("NotoSansKR-Bold.ttf", {"wght": 700}, "Noto Sans KR", 700),
    ]),
    ("ofl/notosansjp/NotoSansJP%5Bwght%5D.ttf", [
        ("NotoSansJP-Regular.ttf", {"wght": 400}, "Noto Sans JP", 400),
        ("NotoSansJP-Bold.ttf", {"wght": 700}, "Noto Sans JP", 700),
    ]),
    ("ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf", [
        ("NotoSansSC-Regular.ttf", {"wght": 400}, "Noto Sans SC", 400),
        ("NotoSansSC-Bold.ttf", {"wght": 700}, "Noto Sans SC", 700),
    ]),
    ("ofl/notosanssymbols/NotoSansSymbols%5Bwght%5D.ttf", [
        ("NotoSansSymbols-Regular.ttf", {"wght": 400}, "Noto Sans Symbols", 400),
    ]),
]

STATIC_FONTS = [
    ("ofl/notosansmath/NotoSansMath-Regular.ttf", "NotoSansMath-Regular.ttf"),
    ("ofl/notosanssymbols2/NotoSansSymbols2-Regular.ttf", "NotoSansSymbols2-Regular.ttf"),
]


def download(url_path: str, dest: str) -> None:
    url = f"{GITHUB_RAW}/{url_path}"
    print(f"  Downloading {url}")
    urllib.request.urlretrieve(url, dest)


def fix_metadata(font: TTFont, family_name: str, weight: int) -> None:
    """Fix name table, OS/2 fsSelection, and head macStyle after instantiation."""
    is_bold = weight >= 700
    style_name = "Bold" if is_bold else "Regular"
    ps_family = family_name.replace(" ", "")
    full_name = f"{family_name} {style_name}"
    ps_name = f"{ps_family}-{style_name}"

    name_table = font["name"]
    # Clear all existing name records and rebuild
    name_table.names = []
    for plat_id, enc_id, lang_id in [(1, 0, 0), (3, 1, 0x0409)]:
        name_table.setName(family_name, 1, plat_id, enc_id, lang_id)
        name_table.setName(style_name, 2, plat_id, enc_id, lang_id)
        name_table.setName(full_name, 4, plat_id, enc_id, lang_id)
        name_table.setName("Version 2.004", 5, plat_id, enc_id, lang_id)
        name_table.setName(ps_name, 6, plat_id, enc_id, lang_id)

    os2 = font["OS/2"]
    os2.usWeightClass = weight
    # fsSelection: bit 5=BOLD, bit 6=REGULAR, bit 7=USE_TYPO_METRICS
    if is_bold:
        os2.fsSelection = (os2.fsSelection | 0x0020 | 0x0080) & ~0x0040
    else:
        os2.fsSelection = (os2.fsSelection | 0x0040 | 0x0080) & ~0x0020

    head = font["head"]
    head.macStyle = 0x0001 if is_bold else 0x0000


def instantiate(vf_path: str, output_path: str, axes: dict[str, float],
                family_name: str, weight: int) -> None:
    font = TTFont(vf_path)
    instantiateVariableFont(font, axes, inplace=True, overlap=True)
    fix_metadata(font, family_name, weight)
    font.save(output_path)

    size_kb = os.path.getsize(output_path) / 1024
    cps = len(font.getBestCmap())
    font.close()
    print(f"  {os.path.basename(output_path)}: {cps} codepoints, weight={weight}, {size_kb:.0f}KB")


def main() -> None:
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <output-dir>")
        sys.exit(1)

    output_dir = sys.argv[1]
    os.makedirs(output_dir, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="noto-vf-") as tmp:
        for url_path, instances in VARIABLE_FONTS:
            vf_name = url_path.split("/")[-1]
            vf_local = os.path.join(tmp, vf_name)

            print(f"\n{vf_name}")
            download(url_path, vf_local)

            for output_name, axes, family_name, weight in instances:
                output_path = os.path.join(output_dir, output_name)
                axes_str = ", ".join(f"{k}={v}" for k, v in axes.items())
                print(f"  Instantiating {output_name} ({axes_str})")
                instantiate(vf_local, output_path, axes, family_name, weight)

        print("\nStatic fonts (direct download)")
        for url_path, output_name in STATIC_FONTS:
            output_path = os.path.join(output_dir, output_name)
            download(url_path, output_path)
            size_kb = os.path.getsize(output_path) / 1024
            font = TTFont(output_path)
            cps = len(font.getBestCmap())
            font.close()
            print(f"  {output_name}: {cps} codepoints, {size_kb:.0f}KB")

    print("\nDone.")


if __name__ == "__main__":
    main()
