#!/usr/bin/env python3
"""Build a CBDT/CBLC color emoji TTF from Twemoji SVG assets.

Usage:
    python3 build-twemoji-font.py <svg-dir> [--output Twemoji.ttf] [--size 128]

Requirements:
    pip install fonttools
    brew install resvg  # or: cargo install resvg
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from fontTools import ttLib
from fontTools.fontBuilder import FontBuilder

# Variation Selector 16
VS16 = 0xFE0F


def parse_filename(name: str) -> list[int] | None:
    """Parse a Twemoji filename like '1f468-200d-1f9b0.svg' into codepoint list."""
    stem = Path(name).stem
    try:
        return [int(part, 16) for part in stem.split("-")]
    except ValueError:
        return None


def collect_svgs(svg_dir: str) -> list[tuple[list[int], str]]:
    """Collect all SVG files and their codepoint sequences. Returns [(codepoints, path)]."""
    entries = []
    for name in sorted(os.listdir(svg_dir)):
        if not name.endswith(".svg"):
            continue
        cps = parse_filename(name)
        if cps is None:
            print(f"  WARN: skipping unparseable filename: {name}", file=sys.stderr)
            continue
        entries.append((cps, os.path.join(svg_dir, name)))
    return entries


def rasterize_svg(svg_path: str, png_path: str, size: int, padding: float) -> bool:
    """Rasterize a single SVG to PNG using resvg.

    Padding expands the SVG viewBox so emoji content doesn't touch bitmap edges.
    This is equivalent to NotoColorEmoji's built-in transparent PNG padding
    (~20% of bitmap area). Without it, emoji edges get clipped.
    """
    try:
        input_path = svg_path
        padded_svg = None

        if padding > 0:
            with open(svg_path, "r") as f:
                svg_content = f.read()
            match = re.search(r'viewBox="([^"]*)"', svg_content)
            if match:
                parts = match.group(1).split()
                x, y, w, h = float(parts[0]), float(parts[1]), float(parts[2]), float(parts[3])
                pad_w = w * padding
                pad_h = h * padding
                new_vb = f"{x - pad_w} {y - pad_h} {w + 2 * pad_w} {h + 2 * pad_h}"
                svg_content = svg_content.replace(match.group(0), f'viewBox="{new_vb}"')
                padded_svg = png_path + ".svg"
                with open(padded_svg, "w") as f:
                    f.write(svg_content)
                input_path = padded_svg

        subprocess.run(
            ["resvg", input_path, png_path, "-w", str(size), "-h", str(size)],
            check=True,
            capture_output=True,
        )
        if padded_svg and os.path.exists(padded_svg):
            os.unlink(padded_svg)
        return True
    except (subprocess.CalledProcessError, FileNotFoundError):
        return False


def rasterize_all(
    entries: list[tuple[list[int], str]], png_dir: str, size: int, jobs: int, padding: float
) -> list[tuple[list[int], str]]:
    """Rasterize all SVGs to PNGs in parallel. Returns [(codepoints, png_path)] for successes."""
    results = []
    failed = []

    def task(cps, svg_path):
        stem = Path(svg_path).stem
        png_path = os.path.join(png_dir, f"{stem}.png")
        ok = rasterize_svg(svg_path, png_path, size, padding)
        return cps, png_path, svg_path, ok

    with ThreadPoolExecutor(max_workers=jobs) as pool:
        futures = [pool.submit(task, cps, svg) for cps, svg in entries]
        for i, future in enumerate(as_completed(futures), 1):
            cps, png_path, svg_path, ok = future.result()
            if ok:
                results.append((cps, png_path))
            else:
                failed.append(Path(svg_path).name)
            if i % 500 == 0 or i == len(futures):
                print(f"  Rasterized {i}/{len(futures)} SVGs...")

    if failed:
        print(f"  WARN: {len(failed)} SVGs failed to rasterize:", file=sys.stderr)
        for name in failed[:20]:
            print(f"    {name}", file=sys.stderr)
        if len(failed) > 20:
            print(f"    ... and {len(failed) - 20} more", file=sys.stderr)

    print(f"  {len(results)} succeeded, {len(failed)} failed")
    return results


def build_glyph_map(
    rasterized: list[tuple[list[int], str]],
) -> tuple[
    dict[int, tuple[int, str]],      # glyph_id -> (glyph_id, png_path) for all glyphs with bitmaps
    dict[int, int],                   # codepoint -> glyph_id (cmap format 12)
    list[tuple[list[int], int]],      # [(codepoint_sequence, glyph_id)] for GSUB ligatures
    list[tuple[int, int]],            # [(base_codepoint, glyph_id)] for cmap format 14 VS16
    int,                              # total glyph count (num_glyphs)
]:
    """Assign glyph IDs and build cmap/GSUB input data.

    Glyph 0 = .notdef (reserved).
    Single-codepoint emoji: glyph IDs 1..N, entered in cmap format 12.
    Multi-codepoint emoji: glyph IDs N+1..M, entered via GSUB ligatures.
    Component codepoints of multi-codepoint sequences that don't already have
    a cmap entry get a placeholder glyph (no bitmap).
    """
    next_gid = 1
    cmap12: dict[int, int] = {}
    vs16_entries: list[tuple[int, int]] = []
    ligatures: list[tuple[list[int], int]] = []
    glyph_bitmaps: dict[int, tuple[int, str]] = {}  # gid -> (gid, png_path)

    singles = [(cps, p) for cps, p in rasterized if len(cps) == 1]
    multis = [(cps, p) for cps, p in rasterized if len(cps) > 1]

    # Phase A: assign glyph IDs for single-codepoint emoji
    for cps, png_path in sorted(singles, key=lambda x: x[0][0]):
        cp = cps[0]
        gid = next_gid
        next_gid += 1
        cmap12[cp] = gid
        glyph_bitmaps[gid] = (gid, png_path)

    # Phase B: assign glyph IDs for multi-codepoint composed emoji
    for cps, png_path in sorted(multis, key=lambda x: x[0]):
        gid = next_gid
        next_gid += 1
        glyph_bitmaps[gid] = (gid, png_path)

        # Strip VS16 for the GSUB ligature sequence (shaper may or may not include it)
        seq_no_vs16 = [cp for cp in cps if cp != VS16]

        # Ensure all component codepoints have cmap entries (placeholder if needed)
        for cp in seq_no_vs16:
            if cp not in cmap12:
                placeholder_gid = next_gid
                next_gid += 1
                cmap12[cp] = placeholder_gid
                # No bitmap for placeholder

        ligatures.append((seq_no_vs16, gid))

        # If the original sequence contains VS16, also register the full sequence
        if VS16 in cps and len(cps) != len(seq_no_vs16):
            # Ensure VS16 itself has a cmap entry
            if VS16 not in cmap12:
                placeholder_gid = next_gid
                next_gid += 1
                cmap12[VS16] = placeholder_gid
            ligatures.append((cps, gid))

        # If sequence is just [base, VS16], add to cmap14 VS16 entries
        if len(cps) == 2 and cps[1] == VS16:
            vs16_entries.append((cps[0], gid))

    total_glyphs = next_gid  # includes .notdef at 0
    print(f"  Glyph IDs assigned: {total_glyphs} total ({len(glyph_bitmaps)} with bitmaps, "
          f"{total_glyphs - len(glyph_bitmaps) - 1} placeholders)")
    print(f"  cmap entries: {len(cmap12)}, ligatures: {len(ligatures)}, VS16 pairs: {len(vs16_entries)}")

    return glyph_bitmaps, cmap12, ligatures, vs16_entries, next_gid


def _build_gsub(
    font: ttLib.TTFont,
    glyph_names: list[str],
    cmap12: dict[int, int],
    ligatures: list[tuple[list[int], int]],
) -> None:
    """Build GSUB table with rlig LigatureSubst for multi-codepoint emoji."""
    from fontTools.ttLib.tables.otTables import (
        GSUB,
        DefaultLangSys,
        Feature,
        FeatureList,
        FeatureRecord,
        Ligature,
        LigatureSubst,
        Lookup,
        LookupList,
        Script,
        ScriptList,
        ScriptRecord,
    )

    # Build ligature mapping: {first_glyph_name: [(remaining_glyph_names, composed_glyph_name)]}
    lig_map: dict[str, list[tuple[list[str], str]]] = {}
    for cps, composed_gid in ligatures:
        gids = []
        valid = True
        for cp in cps:
            if cp in cmap12:
                gids.append(cmap12[cp])
            else:
                valid = False
                break
        if not valid or len(gids) < 2:
            continue

        first_name = glyph_names[gids[0]]
        rest_names = [glyph_names[g] for g in gids[1:]]
        composed_name = glyph_names[composed_gid]

        if first_name not in lig_map:
            lig_map[first_name] = []
        lig_map[first_name].append((rest_names, composed_name))

    if not lig_map:
        return

    # Sort ligatures: longer sequences first (more specific match)
    for first in lig_map:
        lig_map[first].sort(key=lambda x: -len(x[0]))

    # Build LigatureSubst subtable
    lig_subst = LigatureSubst()
    lig_subst.ligatures = {}
    for first_name, ligs in sorted(lig_map.items()):
        lig_list = []
        for rest_names, composed_name in ligs:
            lig = Ligature()
            lig.LigGlyph = composed_name
            lig.Component = rest_names
            lig_list.append(lig)
        lig_subst.ligatures[first_name] = lig_list

    lookup = Lookup()
    lookup.LookupType = 4  # LigatureSubst
    lookup.LookupFlag = 0
    lookup.SubTable = [lig_subst]
    lookup.SubTableCount = 1

    # GSUB structure
    gsub = GSUB()
    gsub.Version = 0x00010000

    feature = Feature()
    feature.FeatureParams = None
    feature.LookupListIndex = [0]
    feature.LookupCount = 1

    feat_record = FeatureRecord()
    feat_record.FeatureTag = "rlig"
    feat_record.Feature = feature

    feat_list = FeatureList()
    feat_list.FeatureRecord = [feat_record]
    feat_list.FeatureCount = 1

    default_lang = DefaultLangSys()
    default_lang.ReqFeatureIndex = 0xFFFF
    default_lang.FeatureIndex = [0]
    default_lang.FeatureCount = 1

    script = Script()
    script.DefaultLangSys = default_lang
    script.LangSysRecord = []
    script.LangSysCount = 0

    script_record = ScriptRecord()
    script_record.ScriptTag = "DFLT"
    script_record.Script = script

    script_list = ScriptList()
    script_list.ScriptRecord = [script_record]
    script_list.ScriptCount = 1

    lookup_list = LookupList()
    lookup_list.Lookup = [lookup]
    lookup_list.LookupCount = 1

    gsub.ScriptList = script_list
    gsub.FeatureList = feat_list
    gsub.LookupList = lookup_list

    gsub_table = ttLib.newTable("GSUB")
    gsub_table.table = gsub
    font["GSUB"] = gsub_table


def _build_cbdt_cblc(
    font: ttLib.TTFont,
    glyph_names: list[str],
    glyph_bitmaps: dict[int, tuple[int, str]],
    strike_size: int,
    upem: int,
    advance: int = 0,
) -> None:
    """Build CBDT and CBLC tables with PNG bitmap data.

    Uses CBDT format 17 (small metrics + raw PNG data) and CBLC index format 1.
    """
    from fontTools.ttLib.tables.BitmapGlyphMetrics import SmallGlyphMetrics
    from fontTools.ttLib.tables.C_B_D_T_ import (
        cbdt_bitmap_format_17,
        table_C_B_D_T_,
    )
    from fontTools.ttLib.tables.C_B_L_C_ import table_C_B_L_C_
    from fontTools.ttLib.tables.E_B_L_C_ import (
        BitmapSizeTable,
        SbitLineMetrics,
        Strike,
        eblc_index_sub_table_1,
    )

    sorted_entries = sorted(glyph_bitmaps.items())  # by gid
    if not sorted_entries:
        return

    # Read all PNG data
    png_data_map: dict[int, bytes] = {}
    for gid, (_, png_path) in sorted_entries:
        with open(png_path, "rb") as f:
            png_data_map[gid] = f.read()

    # --- CBDT table ---
    cbdt_table = table_C_B_D_T_()
    cbdt_table.version = 3.0

    # Bitmap metrics: ppem = strike_size (1:1 mapping, no scaling tricks).
    # Horizontal spacing comes from hmtx advance (2550) > UPEM (2048).
    # Edge-clipping prevention comes from viewBox padding in the PNG.
    # BearingX centers the bitmap within the pixel advance.
    pixel_advance = round(advance / upem * strike_size) if advance > 0 else strike_size
    bearing_x = round((pixel_advance - strike_size) / 2)  # center bitmap in advance
    # Center bitmap vertically in ascender+descender space
    ascender_px = round(1900 / upem * strike_size)
    descender_px = round(500 / upem * strike_size)
    line_height_px = ascender_px + descender_px
    top_margin = round((line_height_px - strike_size) / 2)
    bearing_y = min(ascender_px - top_margin, 127)  # signed byte max

    # Build bitmap glyph objects
    strike_data: dict[str, cbdt_bitmap_format_17] = {}
    for gid, (_, _) in sorted_entries:
        name = glyph_names[gid]
        glyph = cbdt_bitmap_format_17(b"", font)
        glyph.metrics = SmallGlyphMetrics()
        glyph.metrics.height = strike_size
        glyph.metrics.width = strike_size
        glyph.metrics.BearingX = bearing_x
        glyph.metrics.BearingY = bearing_y
        glyph.metrics.Advance = pixel_advance
        glyph.imageData = png_data_map[gid]
        strike_data[name] = glyph

    cbdt_table.strikeData = [strike_data]

    # --- CBLC table ---
    cblc_table = table_C_B_L_C_()
    cblc_table.version = 3.0

    strike = Strike()
    strike.bitmapSizeTable = BitmapSizeTable()
    bst = strike.bitmapSizeTable

    # SbitLineMetrics
    sbit_ascender = min(round(1900 / upem * strike_size), 127)  # signed byte max
    sbit_descender = -(strike_size - sbit_ascender)

    bst.hori = SbitLineMetrics()
    bst.vert = SbitLineMetrics()
    for lm in (bst.hori, bst.vert):
        lm.ascender = sbit_ascender
        lm.descender = max(sbit_descender, -128)
        lm.widthMax = pixel_advance
        lm.caretSlopeNumerator = 0
        lm.caretSlopeDenominator = 0
        lm.caretOffset = 0
        lm.minOriginSB = 0
        lm.minAdvanceSB = 0
        lm.maxBeforeBL = sbit_ascender
        lm.minAfterBL = 0
        lm.pad1 = 0
        lm.pad2 = 0

    bst.colorRef = 0
    bst.startGlyphIndex = sorted_entries[0][0]
    bst.endGlyphIndex = sorted_entries[-1][0]
    bst.ppemX = strike_size
    bst.ppemY = strike_size
    bst.bitDepth = 32
    bst.flags = 0x01  # horizontal metrics

    # Group consecutive glyph IDs into ranges for IndexSubTable entries
    ranges: list[list[int]] = []
    current_range: list[int] = []
    for gid, _ in sorted_entries:
        if current_range and gid != current_range[-1] + 1:
            ranges.append(current_range)
            current_range = []
        current_range.append(gid)
    if current_range:
        ranges.append(current_range)

    strike.indexSubTables = []
    for gid_range in ranges:
        ist = eblc_index_sub_table_1(b"", font)
        ist.indexFormat = 1
        ist.imageFormat = 17
        ist.firstGlyphIndex = gid_range[0]
        ist.lastGlyphIndex = gid_range[-1]
        ist.names = [glyph_names[gid] for gid in gid_range]
        strike.indexSubTables.append(ist)

    cblc_table.strikes = [strike]

    font["CBDT"] = cbdt_table
    font["CBLC"] = cblc_table


def build_font(
    glyph_bitmaps: dict[int, tuple[int, str]],
    cmap12: dict[int, int],
    ligatures: list[tuple[list[int], int]],
    vs16_entries: list[tuple[int, int]],
    num_glyphs: int,
    strike_size: int,
    output_path: str,
) -> None:
    """Assemble the CBDT/CBLC TTF using fonttools."""
    UPEM = 2048
    ADVANCE = 2550  # wider than UPEM to add inter-emoji padding (matches NotoColorEmoji)
    ASCENDER = 1900  # matches NotoColorEmoji
    DESCENDER = -500

    glyph_names = [".notdef"] + [f"glyph{i:05d}" for i in range(1, num_glyphs)]

    # --- Build base font with FontBuilder ---
    fb = FontBuilder(UPEM, isTTF=True)
    fb.setupGlyphOrder(glyph_names)

    # cmap: format 12 (BMP + SMP)
    cmap_dict = {cp: glyph_names[gid] for cp, gid in cmap12.items()}
    fb.setupCharacterMap(cmap_dict)

    # hmtx: advance wider than UPEM for inter-emoji padding
    metrics = {name: (ADVANCE, 0) for name in glyph_names}
    fb.setupHorizontalMetrics(metrics)

    # hhea / OS/2 vertical metrics
    fb.setupHorizontalHeader(ascent=ASCENDER, descent=DESCENDER)
    fb.setupOS2(
        fsType=0x0000,
        usWeightClass=400,
        sTypoAscender=ASCENDER,
        sTypoDescender=DESCENDER,
        sTypoLineGap=0,
        usWinAscent=ASCENDER,
        usWinDescent=abs(DESCENDER),
        sxHeight=0,
        sCapHeight=0,
    )

    fb.setupNameTable({
        "familyName": "Twemoji",
        "styleName": "Regular",
    })

    fb.setupPost(isFixedPitch=0, keepGlyphNames=False)

    # glyf/loca: .notdef has a rectangle, others are empty
    from fontTools.pens.ttGlyphPen import TTGlyphPen
    glyph_table = {}
    for name in glyph_names:
        pen = TTGlyphPen(None)
        if name == ".notdef":
            # Simple rectangle
            pen.moveTo((200, 0))
            pen.lineTo((200, 1400))
            pen.lineTo((1000, 1400))
            pen.lineTo((1000, 0))
            pen.closePath()
        glyph_table[name] = pen.glyph()
    fb.setupGlyf(glyph_table)

    # gasp
    fb.font["gasp"] = ttLib.newTable("gasp")
    fb.font["gasp"].gaspRange = {0xFFFF: 0x000F}

    font = fb.font

    # --- cmap format 14 for VS16 ---
    if vs16_entries:
        from fontTools.ttLib.tables._c_m_a_p import cmap_format_14

        cmap_table = font["cmap"]
        subtable14 = cmap_format_14(14)
        subtable14.platformID = 0
        subtable14.platEncID = 5
        subtable14.format = 14
        subtable14.length = 0
        subtable14.language = 0xFF
        subtable14.numVarSelectorRecords = 1
        subtable14.uvsDict = {
            VS16: [(cp, glyph_names[gid]) for cp, gid in vs16_entries]
        }
        # cmap_format_14 has no .cmap dict, but OS/2.compile() iterates all
        # cmap subtables expecting one. Provide an empty dict to prevent AttributeError.
        subtable14.cmap = {}
        cmap_table.tables.append(subtable14)

    # --- GSUB: rlig ligatures for multi-codepoint sequences ---
    if ligatures:
        _build_gsub(font, glyph_names, cmap12, ligatures)

    # --- CBDT/CBLC bitmap tables ---
    _build_cbdt_cblc(font, glyph_names, glyph_bitmaps, strike_size, UPEM, ADVANCE)

    font.save(output_path)
    font.close()

    # Verify
    size_mb = os.path.getsize(output_path) / 1024 / 1024
    check = ttLib.TTFont(output_path)
    cps = len(check.getBestCmap())
    check.close()
    print(f"  Output: {output_path} ({size_mb:.1f}MB, {cps} cmap entries)")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Build CBDT emoji font from Twemoji SVGs")
    p.add_argument("svg_dir", help="Path to directory containing Twemoji SVG files")
    p.add_argument("--output", default="Twemoji.ttf", help="Output TTF path (default: Twemoji.ttf)")
    p.add_argument("--size", type=int, default=128, help="Bitmap strike size in pixels (default: 128)")
    p.add_argument("--padding", type=float, default=0.05, help="ViewBox padding ratio (default: 0.05 = 5%%)")
    p.add_argument("--keep-pngs", action="store_true", help="Keep temporary PNG directory")
    p.add_argument("--jobs", "-j", type=int, default=os.cpu_count() or 4, help="Parallel rasterization jobs")
    return p.parse_args()


def main() -> None:
    args = parse_args()

    if not (1 <= args.size <= 255):
        print("ERROR: --size must be between 1 and 255", file=sys.stderr)
        sys.exit(1)

    print(f"Collecting SVGs from {args.svg_dir}...")
    entries = collect_svgs(args.svg_dir)
    print(f"  Found {len(entries)} SVGs")

    if not entries:
        print("ERROR: No SVG files found", file=sys.stderr)
        sys.exit(1)

    # Phase 1: Rasterize
    png_dir = tempfile.mkdtemp(prefix="twemoji-png-")
    try:
        print(f"Rasterizing to {args.size}x{args.size} PNGs (jobs={args.jobs})...")
        rasterized = rasterize_all(entries, png_dir, args.size, args.jobs, args.padding)

        if not rasterized:
            print("ERROR: All rasterizations failed", file=sys.stderr)
            sys.exit(1)

        # Phase 2: Build font
        print("Building font...")
        glyph_bitmaps, cmap12, ligatures, vs16_entries, num_glyphs = build_glyph_map(rasterized)

        build_font(
            glyph_bitmaps=glyph_bitmaps,
            cmap12=cmap12,
            ligatures=ligatures,
            vs16_entries=vs16_entries,
            num_glyphs=num_glyphs,
            strike_size=args.size,
            output_path=args.output,
        )
    finally:
        if not args.keep_pngs:
            shutil.rmtree(png_dir, ignore_errors=True)
        else:
            print(f"  PNGs kept at: {png_dir}")

    print("Done.")


if __name__ == "__main__":
    main()
