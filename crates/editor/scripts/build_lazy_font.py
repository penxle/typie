#!/usr/bin/env python3
import base64
import hashlib
import io
import json
import os
import shutil
import struct
import sys
import urllib.request
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

import boto3
import zstandard
from fontTools.ttLib import TTFont
from fontTools.ttLib.sfnt import SFNTWriter

SCRIPT_DIR = Path(__file__).parent
ASSETS_DIR = SCRIPT_DIR / ".." / "assets"

if len(sys.argv) < 2:
    print("Usage: build_lazy_font.py <source-dir>", file=sys.stderr)
    sys.exit(1)

SOURCE_DIR = Path(sys.argv[1])
OUTPUT_DIR = SOURCE_DIR / "lazy"

MAGIC = b"TPFT"
VERSION = 1

CHUNK_SIZE = 200
S3_BUCKET = "typie-cdn"
S3_PREFIX = "editor/fonts"

PHANTOM_FONTS = [
    "Noto-Phantom",
    "Noto-Phantom-Emoji",
]

SLICING_CACHE_DIR = SCRIPT_DIR / ".slicing_cache"
SLICING_BASE_URL = (
    "https://raw.githubusercontent.com/googlefonts/nam-files/main/slices"
)
SLICING_FILES = {
    "korean": "korean_default.txt",
    "japanese": "japanese_default.txt",
    "simplified-chinese": "simplified-chinese_default.txt",
    "traditional-chinese": "traditional-chinese_default.txt",
    "hongkong-chinese": "hongkong-chinese_default.txt",
}
SLICING_MIN_OVERLAP = 1000


def output_hash(directory: Path) -> str:
    h = hashlib.sha256()
    for f in sorted(directory.rglob("*.bin")):
        h.update(f.read_bytes())
    return h.hexdigest()[:8]


def compress_ranges(codepoints: list[int]) -> list[list[int]]:
    if not codepoints:
        return []
    ranges = []
    start = codepoints[0]
    end = codepoints[0]
    for cp in codepoints[1:]:
        if cp == end + 1:
            end = cp
        else:
            ranges.append([start, end])
            start = cp
            end = cp
    ranges.append([start, end])
    return ranges


def parse_slicing_file(path: Path) -> list[list[int]]:
    groups: list[list[int]] = []
    current: list[int] | None = None
    for line in path.read_text().splitlines():
        stripped = line.strip()
        if stripped == "subsets {":
            current = []
        elif stripped == "}" and current is not None:
            if current:
                groups.append(current)
            current = None
        elif current is not None and stripped.startswith("codepoints:"):
            cp_str = stripped.split(":")[1].split("#")[0].strip()
            current.append(int(cp_str))
    return groups


def load_slicing_strategies() -> dict[str, list[list[int]]]:
    SLICING_CACHE_DIR.mkdir(exist_ok=True)
    strategies: dict[str, list[list[int]]] = {}
    for lang, filename in SLICING_FILES.items():
        cache_path = SLICING_CACHE_DIR / filename
        if not cache_path.exists():
            url = f"{SLICING_BASE_URL}/{filename}"
            print(f"  Downloading slicing data: {lang}")
            urllib.request.urlretrieve(url, cache_path)
        strategies[lang] = parse_slicing_file(cache_path)
    return strategies


SLICING_MIN_JACCARD = 0.5

LOCALE_TO_STRATEGY = {
    "KR": "korean",
    "JP": "japanese",
    "SC": "simplified-chinese",
    "TC": "traditional-chinese",
    "HK": "hongkong-chinese",
}


def find_best_strategy(
    font_name: str, font_cps: set[int], strategies: dict[str, list[list[int]]]
) -> tuple[str, list[list[int]]] | None:
    base = font_name.split("-")[0]
    for locale, strategy_name in LOCALE_TO_STRATEGY.items():
        if base.endswith(locale) and strategy_name in strategies:
            groups = strategies[strategy_name]
            strategy_cps: set[int] = set()
            for group in groups:
                strategy_cps.update(group)
            if len(font_cps & strategy_cps) >= SLICING_MIN_OVERLAP:
                return strategy_name, groups

    best_name: str | None = None
    best_groups: list[list[int]] | None = None
    best_score = 0.0
    for name, groups in strategies.items():
        strategy_cps: set[int] = set()
        for group in groups:
            strategy_cps.update(group)
        overlap = len(font_cps & strategy_cps)
        if overlap < SLICING_MIN_OVERLAP:
            continue
        score = overlap / len(font_cps | strategy_cps)
        if score > best_score:
            best_score = score
            best_name = name
            best_groups = groups
    if best_score < SLICING_MIN_JACCARD:
        return None
    return best_name, best_groups


def chunk_with_strategy(
    font_cps: set[int], strategy: list[list[int]]
) -> list[list[int]]:
    covered: set[int] = set()
    chunks: list[list[int]] = []
    for group in strategy:
        chunk = sorted(cp for cp in group if cp in font_cps)
        if chunk:
            chunks.append(chunk)
            covered.update(chunk)
    remaining = sorted(font_cps - covered)
    for i in range(0, len(remaining), CHUNK_SIZE):
        chunks.append(remaining[i : i + CHUNK_SIZE])
    return chunks


def discover_sources() -> list[tuple[str, Path]]:
    if not SOURCE_DIR.is_dir():
        return []
    return sorted(
        (p.stem, p) for p in SOURCE_DIR.iterdir() if p.suffix == ".ttf"
    )


def resolve_gsub_alternates(
    font: TTFont, name_to_gid: dict[str, int]
) -> dict[int, set[int]]:
    if "GSUB" not in font:
        return {}
    gsub = font["GSUB"].table
    alternates: dict[int, set[int]] = {}
    for feature_record in gsub.FeatureList.FeatureRecord:
        for lookup_idx in feature_record.Feature.LookupListIndex:
            lookup = gsub.LookupList.Lookup[lookup_idx]
            for subtable in lookup.SubTable:
                if hasattr(subtable, "mapping"):
                    for src, dst in subtable.mapping.items():
                        src_gid = name_to_gid.get(src)
                        dst_gid = name_to_gid.get(dst)
                        if src_gid is not None and dst_gid is not None:
                            alternates.setdefault(src_gid, set()).add(dst_gid)
                if hasattr(subtable, "alternates"):
                    for src, alts in subtable.alternates.items():
                        src_gid = name_to_gid.get(src)
                        if src_gid is not None:
                            for alt in alts:
                                alt_gid = name_to_gid.get(alt)
                                if alt_gid is not None:
                                    alternates.setdefault(src_gid, set()).add(alt_gid)
                if hasattr(subtable, "ligatures"):
                    for src, ligs in subtable.ligatures.items():
                        src_gid = name_to_gid.get(src)
                        if src_gid is not None:
                            for lig in ligs:
                                lig_gid = name_to_gid.get(lig.LigGlyph)
                                if lig_gid is not None:
                                    alternates.setdefault(src_gid, set()).add(lig_gid)

    return alternates


def resolve_composite_components(
    glyf_table, glyph_name: str, visited: set[str] | None = None
) -> set[str]:
    if visited is None:
        visited = set()
    glyph = glyf_table.get(glyph_name)
    if glyph is None or not glyph.isComposite():
        return set()
    components = set()
    for comp in glyph.components:
        if comp.glyphName in visited:
            continue
        visited.add(comp.glyphName)
        components.add(comp.glyphName)
        components.update(
            resolve_composite_components(glyf_table, comp.glyphName, visited)
        )
    return components


def parse_offset_pair_entries(
    cblc_raw: bytes,
    data_off: int,
    count: int,
    first_glyph: int,
    image_data_offset: int,
    fmt: str,
    stride: int,
) -> dict[int, tuple[int, int]]:
    entries: dict[int, tuple[int, int]] = {}
    for i in range(count):
        off_a = struct.unpack_from(fmt, cblc_raw, data_off + i * stride)[0]
        off_b = struct.unpack_from(fmt, cblc_raw, data_off + (i + 1) * stride)[0]
        if off_a != off_b:
            gid = first_glyph + i
            entries[gid] = (image_data_offset + off_a, image_data_offset + off_b)
    return entries


def parse_cblc_glyph_offsets(cblc_raw: bytes) -> dict[int, tuple[int, int]]:
    glyph_offsets: dict[int, tuple[int, int]] = {}

    _major, _minor, num_sizes = struct.unpack_from(">HHI", cblc_raw, 0)

    for s in range(num_sizes):
        rec_off = 8 + s * 48
        (
            index_subtable_array_offset,
            _index_tables_size,
            num_index_subtables,
        ) = struct.unpack_from(">III", cblc_raw, rec_off)

        for t in range(num_index_subtables):
            entry_off = index_subtable_array_offset + t * 8
            first_glyph, last_glyph, additional_offset = struct.unpack_from(
                ">HHI", cblc_raw, entry_off
            )

            subtable_off = index_subtable_array_offset + additional_offset
            index_format, _image_format, image_data_offset = struct.unpack_from(
                ">HHI", cblc_raw, subtable_off
            )

            data_off = subtable_off + 8
            count = last_glyph - first_glyph + 1

            if index_format == 1:
                glyph_offsets.update(parse_offset_pair_entries(
                    cblc_raw, data_off, count, first_glyph, image_data_offset, ">I", 4
                ))

            elif index_format == 2:
                image_size = struct.unpack_from(">I", cblc_raw, data_off)[0]
                for i in range(count):
                    gid = first_glyph + i
                    start = image_data_offset + i * image_size
                    glyph_offsets[gid] = (start, start + image_size)

            elif index_format == 3:
                glyph_offsets.update(parse_offset_pair_entries(
                    cblc_raw, data_off, count, first_glyph, image_data_offset, ">H", 2
                ))

            elif index_format == 4:
                num_pairs = struct.unpack_from(">I", cblc_raw, data_off)[0]
                pairs_off = data_off + 4
                for i in range(num_pairs):
                    gid, off_a = struct.unpack_from(
                        ">HH", cblc_raw, pairs_off + i * 4
                    )
                    _next_gid, off_b = struct.unpack_from(
                        ">HH", cblc_raw, pairs_off + (i + 1) * 4
                    )
                    if off_a != off_b:
                        glyph_offsets[gid] = (
                            image_data_offset + off_a,
                            image_data_offset + off_b,
                        )

            elif index_format == 5:
                image_size = struct.unpack_from(">I", cblc_raw, data_off)[0]
                num_glyphs_sparse = struct.unpack_from(
                    ">I", cblc_raw, data_off + 12
                )[0]
                array_off = data_off + 16
                for i in range(num_glyphs_sparse):
                    gid = struct.unpack_from(">H", cblc_raw, array_off + i * 2)[0]
                    start = image_data_offset + i * image_size
                    glyph_offsets[gid] = (start, start + image_size)

    return glyph_offsets


def build_chunk_binary(entries: list[tuple[int, bytes]]) -> bytes:
    buf = bytearray()
    buf.extend(struct.pack(">I", len(entries)))
    for offset, data in entries:
        buf.extend(struct.pack(">I", offset))
        buf.extend(struct.pack(">I", len(data)))
        buf.extend(data)
    return bytes(buf)


def find_table_offset(sfnt_data: bytes, tag: bytes) -> int:
    num_tables = struct.unpack_from(">H", sfnt_data, 4)[0]
    for i in range(num_tables):
        rec = 12 + i * 16
        if sfnt_data[rec : rec + 4] == tag:
            return struct.unpack_from(">I", sfnt_data, rec + 8)[0]
    return 0


def wrap_tpft(compressed: bytes) -> bytes:
    return MAGIC + struct.pack(">H", VERSION) + compressed


def process_font(
    name: str, source_path: Path, output_dir: Path,
    strategies: dict[str, list[list[int]]],
) -> tuple[dict | None, list[tuple[str, str]], str | None]:
    font = TTFont(str(source_path))

    has_glyf = "glyf" in font
    has_cbdt = "CBDT" in font and "CBLC" in font

    glyph_order = font.getGlyphOrder()
    name_to_gid = {n: i for i, n in enumerate(glyph_order)}
    num_glyphs = font["maxp"].numGlyphs

    cp_to_gid: dict[int, int] = {}
    for table in font["cmap"].tables:
        if table.isUnicode():
            for cp, glyph_name in table.cmap.items():
                if glyph_name in name_to_gid:
                    cp_to_gid[cp] = name_to_gid[glyph_name]

    gsub_alternates = resolve_gsub_alternates(font, name_to_gid)

    per_glyph: dict[int, tuple[int, bytes]] = {}
    composite_deps: dict[int, set[int]] = {}
    table_overrides: dict[str, bytes] = {}
    split_tag: bytes | None = None

    if has_cbdt:
        cbdt_raw = font.getTableData("CBDT")
        cblc_raw = font.getTableData("CBLC")

        bitmap_offsets = parse_cblc_glyph_offsets(cblc_raw)
        for gid, (start, end) in bitmap_offsets.items():
            if start < end:
                per_glyph[gid] = (start, cbdt_raw[start:end])

        table_overrides["CBDT"] = b"\x00" * len(cbdt_raw)
        table_overrides["CBLC"] = cblc_raw
        split_tag = b"CBDT"
    elif has_glyf:
        glyf_table = font["glyf"]
        for glyph_name in glyph_order:
            glyph = glyf_table.get(glyph_name)
            if glyph is None or not glyph.isComposite():
                continue
            component_names = resolve_composite_components(glyf_table, glyph_name)
            component_gids = {
                name_to_gid[n] for n in component_names if n in name_to_gid
            }
            if component_gids:
                composite_deps[name_to_gid[glyph_name]] = component_gids

        glyf_raw = font.getTableData("glyf")
        loca_raw = font.getTableData("loca")

        is_long = font["head"].indexToLocFormat == 1
        if is_long:
            offsets = list(struct.unpack_from(f">{num_glyphs + 1}I", loca_raw))
        else:
            offsets = [
                o * 2 for o in struct.unpack_from(f">{num_glyphs + 1}H", loca_raw)
            ]

        for gid in range(num_glyphs):
            start = offsets[gid]
            end = offsets[gid + 1]
            if start < end:
                per_glyph[gid] = (start, glyf_raw[start:end])

        table_overrides["glyf"] = b"\x00" * len(glyf_raw)
        table_overrides["loca"] = loca_raw
        split_tag = b"glyf"

    temp_dir = output_dir / name / "_build"
    if temp_dir.exists():
        shutil.rmtree(temp_dir)
    temp_dir.mkdir(parents=True, exist_ok=True)

    compressor = zstandard.ZstdCompressor()
    strategy_name: str | None = None
    chunk_count = 0
    chunk_map_b64: str | None = None
    chunk_map_sup: list[int] | None = None

    if split_tag is not None:
        all_cps = set(cp_to_gid.keys())
        match = find_best_strategy(name, all_cps, strategies)
        if match:
            strategy_name, strategy_groups = match
            chunks_cp = chunk_with_strategy(all_cps, strategy_groups)
        else:
            strategy_name = None
            sorted_cps = sorted(all_cps)
            chunks_cp = [
                sorted_cps[i : i + CHUNK_SIZE]
                for i in range(0, len(sorted_cps), CHUNK_SIZE)
            ]

        chunk_count = len(chunks_cp)
        chunks_dir = temp_dir / "chunks"
        chunks_dir.mkdir(exist_ok=True)

        cp_to_chunk: dict[int, int] = {}
        for chunk_idx, cps in enumerate(chunks_cp):
            for cp in cps:
                cp_to_chunk[cp] = chunk_idx

            gids_needed: set[int] = set()
            for cp in cps:
                gid = cp_to_gid[cp]
                gids_needed.add(gid)
                if gid in gsub_alternates:
                    gids_needed.update(gsub_alternates[gid])

            expanded: set[int] = set()
            for gid in gids_needed:
                if gid in composite_deps:
                    expanded.update(composite_deps[gid])
            gids_needed.update(expanded)

            entries = [
                per_glyph[gid] for gid in sorted(gids_needed) if gid in per_glyph
            ]

            chunk_data = build_chunk_binary(entries)
            blob = wrap_tpft(compressor.compress(chunk_data))
            (chunks_dir / f"{chunk_idx}.bin").write_bytes(blob)

        if cp_to_chunk:
            bmp = {cp: idx for cp, idx in cp_to_chunk.items() if cp <= 0xFFFF}
            sup = {cp: idx for cp, idx in cp_to_chunk.items() if cp > 0xFFFF}

            if bmp:
                l1 = bytearray([0xFF] * 256)
                l2_pages: list[bytearray] = []
                for page_num in range(256):
                    page_start = page_num << 8
                    page = bytearray([0xFF] * 256)
                    has_entry = False
                    for cp, idx in bmp.items():
                        if page_start <= cp < page_start + 256:
                            page[cp - page_start] = idx
                            has_entry = True
                    if has_entry:
                        l1[page_num] = len(l2_pages)
                        l2_pages.append(page)
                paged = bytearray(l1)
                for page in l2_pages:
                    paged.extend(page)
                chunk_map_b64 = base64.b64encode(bytes(paged)).decode("ascii")

            if sup:
                chunk_map_sup = []
                for cp in sorted(sup):
                    chunk_map_sup.extend([cp, sup[cp]])

    real_tags = list(font.reader.keys())

    buf = io.BytesIO()
    writer = SFNTWriter(buf, len(real_tags), font.sfntVersion)
    for tag in real_tags:
        if tag in table_overrides:
            writer[tag] = table_overrides[tag]
        else:
            writer[tag] = font.reader[tag]
    writer.close()
    base_data = buf.getvalue()

    split_offset = 0
    if split_tag is not None:
        split_offset = find_table_offset(base_data, split_tag)

    base_with_header = struct.pack(">I", split_offset) + base_data
    blob = wrap_tpft(compressor.compress(base_with_header))
    (temp_dir / "base.bin").write_bytes(blob)

    hash_hex = output_hash(temp_dir)
    font_dir = output_dir / name / hash_hex
    if font_dir.exists():
        shutil.rmtree(font_dir)
    temp_dir.rename(font_dir)

    s3_base = f"{S3_PREFIX}/{name}/{hash_hex}"
    s3_files: list[tuple[str, str]] = []
    s3_files.append((str(font_dir / "base.bin"), f"{s3_base}/base.bin"))
    chunks_dir = font_dir / "chunks"
    if chunks_dir.exists():
        for chunk_file in sorted(chunks_dir.iterdir()):
            s3_files.append(
                (str(chunk_file), f"{s3_base}/chunks/{chunk_file.name}")
            )

    manifest_entry: dict = {
        "hash": hash_hex,
        "chunk_count": chunk_count,
        "chunk_map": chunk_map_b64,
    }
    if chunk_map_sup:
        manifest_entry["chunk_map_sup"] = chunk_map_sup

    font.close()
    return manifest_entry, s3_files, strategy_name


def sync_s3(local_files: list[tuple[str, str]]) -> None:
    s3 = boto3.client("s3")

    existing: set[str] = set()
    paginator = s3.get_paginator("list_objects_v2")
    for page in paginator.paginate(Bucket=S3_BUCKET, Prefix=S3_PREFIX + "/"):
        for obj in page.get("Contents", []):
            existing.add(obj["Key"])

    total = len(local_files)
    uploaded = 0
    skipped = 0
    for i, (local_path, s3_key) in enumerate(local_files, 1):
        if s3_key in existing:
            print(f"  [{i}/{total}] SKIP {s3_key}")
            skipped += 1
            continue
        print(f"  [{i}/{total}] UPLOAD {s3_key}")
        s3.upload_file(
            local_path,
            S3_BUCKET,
            s3_key,
            ExtraArgs={"ContentType": "application/octet-stream"},
        )
        uploaded += 1

    print(f"S3: {uploaded} uploaded, {skipped} skipped")


def main():
    strategies = load_slicing_strategies()

    if OUTPUT_DIR.exists():
        shutil.rmtree(OUTPUT_DIR)
    OUTPUT_DIR.mkdir(parents=True)

    tasks = discover_sources()

    for phantom_name in PHANTOM_FONTS:
        source = ASSETS_DIR / f"{phantom_name}.ttf"
        if source.exists():
            tasks.append((phantom_name, source))
        else:
            print(f"  SKIP: {phantom_name} (source not found)", file=sys.stderr)

    workers = min(os.cpu_count() or 4, len(tasks))
    print(f"{len(tasks)} fonts, {workers} workers")

    combined: dict[str, dict] = {}
    all_s3_files: list[tuple[str, str]] = []
    done = 0

    with ProcessPoolExecutor(max_workers=workers) as executor:
        futures = {
            executor.submit(process_font, name, path, OUTPUT_DIR, strategies): name
            for name, path in tasks
        }

        for future in as_completed(futures):
            name = futures[future]
            done += 1
            try:
                manifest_entry, s3_files, used_strategy = future.result()
            except Exception as e:
                print(f"  [{done}/{len(tasks)}] FAIL {name}: {e}")
                continue
            if manifest_entry is None:
                print(f"  [{done}/{len(tasks)}] SKIP {name}")
                continue
            combined[name] = manifest_entry
            all_s3_files.extend(s3_files)
            h = manifest_entry["hash"]
            num_chunks = manifest_entry["chunk_count"]
            chunk_dir = OUTPUT_DIR / name / h / "chunks"
            base_size = (OUTPUT_DIR / name / h / "base.bin").stat().st_size
            total_chunk_size = sum(
                (chunk_dir / f"{i}.bin").stat().st_size
                for i in range(num_chunks)
            )
            strategy_label = used_strategy or "sequential"
            print(
                f"  [{done}/{len(tasks)}] {name} "
                f"base: {base_size / 1024:.1f}KB, "
                f"{num_chunks} chunks: {total_chunk_size / 1024:.1f}KB, "
                f"strategy: {strategy_label}"
            )

    manifest_path = ASSETS_DIR / "fonts.json"
    manifest_path.write_text(json.dumps(combined, separators=(",", ":"), sort_keys=True))

    print(f"\nManifest: {manifest_path}")
    print(f"Fonts: {len(combined)}")

    sync_s3(all_s3_files)


if __name__ == "__main__":
    main()
