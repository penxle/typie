import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/service.dart';

class Font {
  const Font({required this.id, required this.weight, this.subfamilyDisplayName, required this.url, this.state});

  final String id;
  final int weight;
  final String? subfamilyDisplayName;
  final String url;
  final String? state;
}

class FontFamily {
  const FontFamily({
    required this.id,
    required this.familyName,
    required this.displayName,
    required this.fonts,
    this.state,
  });

  final String id;
  final String familyName;
  final String displayName;
  final List<Font> fonts;
  final String? state;
}

Font? getRepresentativeFont(List<Font> fonts) {
  final active = fonts.where((f) => f.state == 'ACTIVE').toList();
  if (active.isEmpty) {
    return null;
  }
  return active.reduce((prev, curr) {
    final prevDiff = (prev.weight - 400).abs();
    final currDiff = (curr.weight - 400).abs();
    if (currDiff < prevDiff) {
      return curr;
    }
    if (currDiff == prevDiff && curr.weight > prev.weight) {
      return curr;
    }
    return prev;
  });
}

class FontManifest {
  FontManifest({required this.hash, required this.chunkCount, required this.chunkMap, this.chunkMapSup});

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

const _cdnBase = 'https://cdn.typie.net/editor/fonts';
const _fontCacheDir = 'fonts';
const _phantomFontFamilies = [
  (familyName: 'Noto (Phantom)', path: 'assets/native/Noto-Phantom.bin'),
  (familyName: 'Noto Emoji (Phantom)', path: 'assets/native/Noto-Phantom-Emoji.bin'),
];

String? _cacheBasePath;

Future<String> _getCacheBasePath() async {
  if (_cacheBasePath != null) {
    return _cacheBasePath!;
  }
  final cacheDir = await getApplicationCacheDirectory();
  _cacheBasePath = '${cacheDir.path}/$_fontCacheDir';
  await Directory(_cacheBasePath!).create(recursive: true);
  return _cacheBasePath!;
}

class FontManager {
  FontManager(this._app);

  final NativeEditorApplication _app;
  final _loaded = <String>{};
  final _pending = <String, Future<void>>{};
  final _fetching = <String, Future<Uint8List>>{};
  final _manifestCache = <String, FontManifest>{};
  final _manifestFetching = <String, Future<FontManifest>>{};
  static const _preloadConcurrency = 4;
  final _preloadPending = <_PreloadItem>[];
  int _preloadInflight = 0;
  final _preloadPromises = <String, Future<void>>{};
  List<FontFamily>? _fallbackFontFamilies;
  Future<List<FontFamily>>? _fallbacksLoading;

  List<FontFamily> fontFamilies = [];

  Future<void> _preloadEnqueue(String key, double priority, Future<void> Function() fn) {
    if (_loaded.contains(key)) {
      return Future.value();
    }

    final existing = _preloadPromises[key];
    if (existing != null) {
      return existing;
    }

    final completer = Completer<void>();
    final item = _PreloadItem(key: key, priority: priority, fn: fn, completer: completer);
    var i = _preloadPending.indexWhere((p) => p.priority < priority);
    if (i == -1) {
      i = _preloadPending.length;
    }
    _preloadPending.insert(i, item);

    _preloadPromises[key] = completer.future;
    _preloadFlush();
    return completer.future;
  }

  void _preloadFlush() {
    while (_preloadInflight < _preloadConcurrency && _preloadPending.isNotEmpty) {
      final item = _preloadPending.removeAt(0);

      if (_loaded.contains(item.key)) {
        unawaited(_preloadPromises.remove(item.key));
        item.completer.complete();
        continue;
      }

      _preloadInflight++;
      unawaited(
        item
            .fn()
            .then((_) {
              unawaited(_preloadPromises.remove(item.key));
              item.completer.complete();
              _preloadInflight--;
              _preloadFlush();
            })
            .catchError((Object err) {
              unawaited(_preloadPromises.remove(item.key));
              item.completer.completeError(err);
              _preloadInflight--;
              _preloadFlush();
            }),
      );
    }
  }

  Font? findFont(String family, int weight) {
    final familyFonts = fontFamilies.where((f) => f.familyName == family).expand((f) => f.fonts).toList();
    if (familyFonts.isEmpty) {
      return null;
    }

    final exact = familyFonts.where((f) => f.weight == weight).firstOrNull;
    if (exact != null) {
      return exact;
    }

    // CSS Fonts Level 4 §5.2 font-weight matching
    final sorted = [...familyFonts]..sort((a, b) => a.weight.compareTo(b.weight));
    if (weight >= 400 && weight <= 500) {
      return sorted.where((f) => f.weight >= weight && f.weight <= 500).firstOrNull ??
          sorted.where((f) => f.weight < weight).lastOrNull ??
          sorted.where((f) => f.weight > 500).firstOrNull;
    } else if (weight < 400) {
      return sorted.where((f) => f.weight <= weight).lastOrNull ?? sorted.where((f) => f.weight > weight).firstOrNull;
    } else {
      return sorted.where((f) => f.weight >= weight).firstOrNull ?? sorted.where((f) => f.weight < weight).lastOrNull;
    }
  }

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

  Future<FontManifest> _fetchManifest(String url) {
    final cached = _manifestCache[url];
    if (cached != null) {
      return Future.value(cached);
    }

    final inflight = _manifestFetching[url];
    if (inflight != null) {
      return inflight;
    }

    final future = () async {
      try {
        final response = await serviceLocator<Dio>().get<String>('$url/manifest.json');
        final v = jsonDecode(response.data!) as Map<String, dynamic>;
        final manifest = FontManifest(
          hash: v['hash'] as String,
          chunkCount: v['chunk_count'] as int,
          chunkMap: v['chunk_map'] as String?,
          chunkMapSup: (v['chunk_map_sup'] as List<dynamic>?)?.cast<int>(),
        );
        _manifestCache[url] = manifest;
        return manifest;
      } finally {
        unawaited(_manifestFetching.remove(url));
      }
    }();

    _manifestFetching[url] = future;
    return future;
  }

  Future<List<FontFamily>> _loadFallbackFontFamilies() {
    if (_fallbackFontFamilies != null) {
      return Future.value(_fallbackFontFamilies!);
    }
    if (_fallbacksLoading != null) {
      return _fallbacksLoading!;
    }

    final future = () async {
      try {
        final raw = await rootBundle.loadString('assets/native/fallbacks.json');
        final data = jsonDecode(raw) as List<dynamic>;
        final families = <FontFamily>[];
        for (final entry in data) {
          final e = entry as Map<String, dynamic>;
          final familyName = e['familyName'] as String;
          final fonts = <Font>[];
          for (final f in e['fonts'] as List<dynamic>) {
            final fm = f as Map<String, dynamic>;
            final url = '$_cdnBase/${fm['path'] as String}';
            fonts.add(Font(id: '', weight: fm['weight'] as int, url: url));
            _manifestCache[url] = FontManifest(
              hash: fm['hash'] as String,
              chunkCount: fm['chunk_count'] as int,
              chunkMap: fm['chunk_map'] as String?,
              chunkMapSup: (fm['chunk_map_sup'] as List<dynamic>?)?.cast<int>(),
            );
          }
          families.add(
            FontFamily(
              id: '',
              familyName: familyName,
              displayName: (e['displayName'] as String?) ?? familyName,
              fonts: fonts,
            ),
          );
        }
        _fallbackFontFamilies = families;
        return families;
      } finally {
        _fallbacksLoading = null;
      }
    }();

    _fallbacksLoading = future;
    return future;
  }

  Future<void> _loadOnce(String key, Future<void> Function() fn) {
    if (_loaded.contains(key)) {
      return Future.value();
    }

    final existing = _pending[key];
    if (existing != null) {
      return existing;
    }

    final future = () async {
      try {
        await fn();
        _loaded.add(key);
      } finally {
        unawaited(_pending.remove(key));
      }
    }();

    _pending[key] = future;
    return future;
  }

  int _lookupChunkIndex(Uint8List data, List<int>? sup, int cp) {
    if (cp <= 0xFFFF) {
      final l2 = data[cp >> 8];
      if (l2 == 0xFF) {
        return -1;
      }
      final chunk = data[256 + l2 * 256 + (cp & 0xFF)];
      return chunk == 0xFF ? -1 : chunk;
    }
    if (sup != null) {
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
    }
    return -1;
  }

  bool _hasCodepoint(FontManifest manifest, int cp) {
    final data = manifest.decodedChunkMap;
    if (data == null) {
      return false;
    }
    return _lookupChunkIndex(data, manifest.chunkMapSup, cp) >= 0;
  }

  List<int> _findChunkIndices(FontManifest manifest, List<int> codepoints) {
    final data = manifest.decodedChunkMap;
    if (data == null) {
      return [];
    }
    final indices = <int>{};
    for (final cp in codepoints) {
      final idx = _lookupChunkIndex(data, manifest.chunkMapSup, cp);
      if (idx >= 0) {
        indices.add(idx);
      }
    }
    return indices.toList();
  }

  Future<void> _loadBase(String family, Font font) async {
    await _loadOnce('base:$family:${font.weight}', () async {
      final manifest = await _fetchManifest(font.url);
      final data = await _fetchFont('${font.url}/${manifest.hash}/base.bin');
      _app.addFontBase(family, font.weight, data);
    });
  }

  Future<void> _loadChunks(String family, Font font, List<int> codepoints) async {
    final manifest = await _fetchManifest(font.url);

    await Future.wait(
      _findChunkIndices(manifest, codepoints)
          .map(
            (idx) => _loadOnce('chunk:$family:${font.weight}:$idx', () async {
              final data = await _fetchFont('${font.url}/${manifest.hash}/chunks/$idx.bin');
              _app.addFontChunk(family, font.weight, data);
            }),
          )
          .map((f) => f.catchError((_) {})),
    );
  }

  Future<void> initFonts() async {
    await Future.wait(
      _phantomFontFamilies.map((f) async {
        final data = await rootBundle.load(f.path);
        _app.addFontBase(f.familyName, 400, data.buffer.asUint8List());
      }),
    );

    await _loadFallbackFontFamilies();
  }

  Future<List<int>> filterUncoveredCodepoints(Font font, List<int> codepoints) async {
    final manifest = await _fetchManifest(font.url);
    return codepoints.where((cp) => !_hasCodepoint(manifest, cp)).toList();
  }

  Future<void> ensureRequiredFont(String family, Font font, List<int> codepoints) async {
    await _loadBase(family, font);
    await _loadChunks(family, font, codepoints);
  }

  Future<void> preloadRemainingChunks(String family, Font font) async {
    try {
      final manifest = await _fetchManifest(font.url);
      for (var i = manifest.chunkCount - 1; i >= 0; i--) {
        final key = 'chunk:$family:${font.weight}:$i';
        if (!_loaded.contains(key)) {
          final idx = i; // capture for closure
          unawaited(
            _preloadEnqueue(key, idx / manifest.chunkCount, () async {
              try {
                await _loadOnce(key, () async {
                  final data = await _fetchFont('${font.url}/${manifest.hash}/chunks/$idx.bin');
                  _app.addFontChunk(family, font.weight, data);
                });
              } catch (_) {
                // best-effort: silently ignore preload failures
              }
            }),
          );
        }
      }
    } catch (_) {
      // best-effort
    }
  }

  Future<List<Map<String, dynamic>>> resolveFallbackMappings(int weight, List<int> uncovered) async {
    final fallbacks = await _loadFallbackFontFamilies();
    final mappings = <Map<String, dynamic>>[];
    var remaining = uncovered;

    for (final fallbackFontFamily in fallbacks) {
      if (remaining.isEmpty) {
        break;
      }

      if (fallbackFontFamily.fonts.isEmpty) {
        continue;
      }
      // CSS Fonts Level 4 §5.2 font-weight matching
      final sorted = [...fallbackFontFamily.fonts]..sort((a, b) => a.weight.compareTo(b.weight));
      final Font? fallbackFont;
      if (weight >= 400 && weight <= 500) {
        fallbackFont =
            sorted.where((f) => f.weight >= weight && f.weight <= 500).firstOrNull ??
            sorted.where((f) => f.weight < weight).lastOrNull ??
            sorted.where((f) => f.weight > 500).firstOrNull;
      } else if (weight < 400) {
        fallbackFont =
            sorted.where((f) => f.weight <= weight).lastOrNull ?? sorted.where((f) => f.weight > weight).firstOrNull;
      } else {
        fallbackFont =
            sorted.where((f) => f.weight >= weight).firstOrNull ?? sorted.where((f) => f.weight < weight).lastOrNull;
      }
      if (fallbackFont == null) {
        continue;
      }

      final manifest = _manifestCache[fallbackFont.url];
      if (manifest == null) {
        continue;
      }

      final covered = remaining.where((cp) => _hasCodepoint(manifest, cp)).toList();
      if (covered.isEmpty) {
        continue;
      }

      await _loadBase(fallbackFontFamily.familyName, fallbackFont);
      await _loadChunks(fallbackFontFamily.familyName, fallbackFont, covered);

      mappings.add({'family': fallbackFontFamily.familyName, 'weight': fallbackFont.weight, 'codepoints': covered});

      final coveredSet = covered.toSet();
      remaining = remaining.where((cp) => !coveredSet.contains(cp)).toList();
    }

    return mappings;
  }
}

class _PreloadItem {
  _PreloadItem({required this.key, required this.priority, required this.fn, required this.completer});

  final String key;
  final double priority;
  final Future<void> Function() fn;
  final Completer<void> completer;
}
