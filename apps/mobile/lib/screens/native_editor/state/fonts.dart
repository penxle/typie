import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/service.dart';

class FontVariant {
  const FontVariant({required this.weight, required this.path});

  final int weight;
  final String path;
}

class FontConfig {
  const FontConfig({required this.family, required this.variants});

  final String family;
  final List<FontVariant> variants;
}

class FallbackFontConfig extends FontConfig {
  const FallbackFontConfig({required super.family, required this.priority, required super.variants});

  final int priority;
}

class FontManifestEntry {
  FontManifestEntry({required this.hash, required this.chunkCount, required this.chunkMap, this.chunkMapSup});

  final String hash;
  final int chunkCount;
  final String? chunkMap;
  final List<int>? chunkMapSup;

  Uint8List? _decodedMap;

  Uint8List? get decodedChunkMap {
    if (chunkMap == null) {
      return null;
    }
    return _decodedMap ??= base64Decode(chunkMap!);
  }
}

const cdnBase = 'https://cdn.typie.net/editor/fonts';
const _fontCacheDir = 'fonts';

const defaultFonts = <FontConfig>[
  FontConfig(
    family: 'Pretendard',
    variants: [
      FontVariant(weight: 100, path: 'Pretendard-Thin'),
      FontVariant(weight: 200, path: 'Pretendard-ExtraLight'),
      FontVariant(weight: 300, path: 'Pretendard-Light'),
      FontVariant(weight: 400, path: 'Pretendard-Regular'),
      FontVariant(weight: 500, path: 'Pretendard-Medium'),
      FontVariant(weight: 600, path: 'Pretendard-SemiBold'),
      FontVariant(weight: 700, path: 'Pretendard-Bold'),
      FontVariant(weight: 800, path: 'Pretendard-ExtraBold'),
      FontVariant(weight: 900, path: 'Pretendard-Black'),
    ],
  ),
  FontConfig(
    family: 'KoPubWorldDotum',
    variants: [
      FontVariant(weight: 300, path: 'KoPubWorldDotum-Light'),
      FontVariant(weight: 500, path: 'KoPubWorldDotum-Medium'),
      FontVariant(weight: 700, path: 'KoPubWorldDotum-Bold'),
    ],
  ),
  FontConfig(
    family: 'NanumBarunGothic',
    variants: [
      FontVariant(weight: 200, path: 'NanumBarunGothic-UltraLight'),
      FontVariant(weight: 300, path: 'NanumBarunGothic-Light'),
      FontVariant(weight: 400, path: 'NanumBarunGothic-Regular'),
      FontVariant(weight: 700, path: 'NanumBarunGothic-Bold'),
    ],
  ),
  FontConfig(
    family: 'RIDIBatang',
    variants: [FontVariant(weight: 400, path: 'RIDIBatang-Regular')],
  ),
  FontConfig(
    family: 'KoPubWorldBatang',
    variants: [
      FontVariant(weight: 300, path: 'KoPubWorldBatang-Light'),
      FontVariant(weight: 500, path: 'KoPubWorldBatang-Medium'),
      FontVariant(weight: 700, path: 'KoPubWorldBatang-Bold'),
    ],
  ),
  FontConfig(
    family: 'NanumMyeongjo',
    variants: [
      FontVariant(weight: 400, path: 'NanumMyeongjo-Regular'),
      FontVariant(weight: 700, path: 'NanumMyeongjo-Bold'),
      FontVariant(weight: 800, path: 'NanumMyeongjo-ExtraBold'),
    ],
  ),
];

const _fallbackFonts = <FallbackFontConfig>[
  FallbackFontConfig(
    family: 'Pretendard (Fallback)',
    priority: 100,
    variants: [FontVariant(weight: 400, path: 'Pretendard-Regular')],
  ),
  FallbackFontConfig(
    family: 'Noto Sans JP',
    priority: 200,
    variants: [
      FontVariant(weight: 400, path: 'NotoSansJP-Regular'),
      FontVariant(weight: 700, path: 'NotoSansJP-Bold'),
    ],
  ),
  FallbackFontConfig(
    family: 'Noto Sans SC',
    priority: 300,
    variants: [
      FontVariant(weight: 400, path: 'NotoSansSC-Regular'),
      FontVariant(weight: 700, path: 'NotoSansSC-Bold'),
    ],
  ),
  FallbackFontConfig(
    family: 'NotoColorEmoji',
    priority: 400,
    variants: [FontVariant(weight: 400, path: 'NotoColorEmoji')],
  ),
  FallbackFontConfig(
    family: 'Noto (Phantom)',
    priority: 65534,
    variants: [FontVariant(weight: 400, path: 'Noto-Phantom')],
  ),
  FallbackFontConfig(
    family: 'Noto Emoji (Phantom)',
    priority: 65535,
    variants: [FontVariant(weight: 400, path: 'Noto-Phantom-Emoji')],
  ),
];

const _allFonts = <FontConfig>[...defaultFonts, ..._fallbackFonts];

String? _cacheBasePath;
Map<String, FontManifestEntry>? _manifest;

Future<String> _getCacheBasePath() async {
  if (_cacheBasePath != null) {
    return _cacheBasePath!;
  }
  final cacheDir = await getApplicationCacheDirectory();
  _cacheBasePath = '${cacheDir.path}/$_fontCacheDir';
  await Directory(_cacheBasePath!).create(recursive: true);
  return _cacheBasePath!;
}

Future<Map<String, FontManifestEntry>> _getManifest() async {
  if (_manifest != null) {
    return _manifest!;
  }
  final jsonStr = await rootBundle.loadString('assets/native/fonts.json');
  final raw = jsonDecode(jsonStr) as Map<String, dynamic>;
  _manifest = raw.map((key, value) {
    final v = value as Map<String, dynamic>;
    return MapEntry(
      key,
      FontManifestEntry(
        hash: v['hash'] as String,
        chunkCount: v['chunk_count'] as int,
        chunkMap: v['chunk_map'] as String?,
        chunkMapSup: (v['chunk_map_sup'] as List<dynamic>?)?.cast<int>(),
      ),
    );
  });
  return _manifest!;
}

class FontManager {
  FontManager(this._app);

  final NativeEditorApplication _app;
  final _loaded = <String>{};
  final _pending = <String, Future<void>>{};
  final _fetching = <String, Future<Uint8List>>{};

  Future<Uint8List> _fetchFont(String url) async {
    final basePath = await _getCacheBasePath();
    final cacheKey = url.replaceAll('/', '_');
    final cacheFile = File('$basePath/$cacheKey');

    if (cacheFile.existsSync()) {
      return cacheFile.readAsBytes();
    }

    final inflight = _fetching[url];
    if (inflight != null) {
      return inflight;
    }

    final future = () async {
      try {
        final response = await serviceLocator<Dio>().get<List<int>>(
          url,
          options: Options(responseType: ResponseType.bytes),
        );
        final data = Uint8List.fromList(response.data!);
        unawaited(cacheFile.writeAsBytes(data));
        return data;
      } finally {
        unawaited(_fetching.remove(url));
      }
    }();

    _fetching[url] = future;
    return future;
  }

  Future<void> _loadOnce(String key, Future<void> Function() fn) async {
    if (_loaded.contains(key)) {
      return;
    }

    final existing = _pending[key];
    if (existing != null) {
      return existing;
    }

    final future = () async {
      await fn();
      _loaded.add(key);
    }();

    _pending[key] = future;
    try {
      await future;
    } finally {
      unawaited(_pending.remove(key));
    }
  }

  List<int> _findChunkIndices(FontManifestEntry fm, List<int> codepoints) {
    final data = fm.decodedChunkMap;
    if (data == null) {
      return [];
    }

    final indices = <int>{};
    for (final cp in codepoints) {
      if (cp <= 0xFFFF) {
        final l2Idx = data[cp >> 8];
        if (l2Idx == 0xFF) {
          continue;
        }
        final chunk = data[256 + l2Idx * 256 + (cp & 0xFF)];
        if (chunk != 0xFF) {
          indices.add(chunk);
        }
      } else if (fm.chunkMapSup != null) {
        final idx = _findSupplementaryChunk(fm.chunkMapSup!, cp);
        if (idx >= 0) {
          indices.add(idx);
        }
      }
    }
    return indices.toList();
  }

  int _findSupplementaryChunk(List<int> sup, int cp) {
    var lo = 0;
    var hi = sup.length ~/ 2 - 1;
    while (lo <= hi) {
      final mid = (lo + hi) ~/ 2;
      final key = sup[mid * 2];
      if (cp < key) {
        hi = mid - 1;
      } else if (cp > key) {
        lo = mid + 1;
      } else {
        return sup[mid * 2 + 1];
      }
    }
    return -1;
  }

  Future<void> ensureAllFontBases() async {
    final manifest = await _getManifest();
    final futures = <Future<void>>[];

    for (final config in _allFonts) {
      for (final variant in config.variants) {
        final fm = manifest[variant.path];
        if (fm == null) {
          continue;
        }

        futures.add(
          _loadOnce('base:${config.family}:${variant.weight}', () async {
            final data = await _fetchFont('$cdnBase/${variant.path}/${fm.hash}/base.bin');
            _app.addFontBase(config.family, variant.weight, data);
          }),
        );
      }
    }

    await Future.wait(futures.map((f) => f.catchError((_) {})));

    final fallbacks = List<FallbackFontConfig>.from(_fallbackFonts)..sort((a, b) => a.priority.compareTo(b.priority));
    _app.setFallbackFonts(fallbacks.map((c) => c.family).toList());
  }

  Future<void> _loadChunks(List<FontConfig> configs, List<int> codepoints) async {
    final manifest = await _getManifest();
    final futures = <Future<void>>[];

    for (final config in configs) {
      for (final variant in config.variants) {
        final fm = manifest[variant.path];
        if (fm == null) {
          continue;
        }

        for (final idx in _findChunkIndices(fm, codepoints)) {
          futures.add(
            _loadOnce('chunk:${config.family}:${variant.weight}:$idx', () async {
              final data = await _fetchFont('$cdnBase/${variant.path}/${fm.hash}/chunks/$idx.bin');
              _app.addFontChunk(config.family, variant.weight, data);
            }),
          );
        }
      }
    }

    await Future.wait(futures.map((f) => f.catchError((_) {})));
  }

  Future<void> ensureRequiredFont(String family, int weight, List<int> codepoints) async {
    final config = defaultFonts.where((c) => c.family == family).firstOrNull;
    final variant = config?.variants.where((v) => v.weight == weight).firstOrNull;
    if (variant == null) {
      return;
    }

    await _loadChunks([
      FontConfig(family: family, variants: [variant]),
    ], codepoints);
  }

  Future<void> ensureRequiredFallbackFont(List<int> codepoints) async {
    await _loadChunks(List<FontConfig>.from(_fallbackFonts), codepoints);
  }
}
