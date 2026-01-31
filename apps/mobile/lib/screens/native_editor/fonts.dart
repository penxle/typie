import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:typie/native/editor_native.dart';

class FontInfo {
  const FontInfo({required this.name, required this.weight, required this.file});

  final String name;
  final int weight;
  final String file;
}

const editorFonts = <FontInfo>[
  FontInfo(name: 'Pretendard', weight: 100, file: 'Pretendard-Thin.ttf'),
  FontInfo(name: 'Pretendard', weight: 200, file: 'Pretendard-ExtraLight.ttf'),
  FontInfo(name: 'Pretendard', weight: 300, file: 'Pretendard-Light.ttf'),
  FontInfo(name: 'Pretendard', weight: 400, file: 'Pretendard-Regular.ttf'),
  FontInfo(name: 'Pretendard', weight: 500, file: 'Pretendard-Medium.ttf'),
  FontInfo(name: 'Pretendard', weight: 600, file: 'Pretendard-SemiBold.ttf'),
  FontInfo(name: 'Pretendard', weight: 700, file: 'Pretendard-Bold.ttf'),
  FontInfo(name: 'Pretendard', weight: 800, file: 'Pretendard-ExtraBold.ttf'),
  FontInfo(name: 'Pretendard', weight: 900, file: 'Pretendard-Black.ttf'),
  FontInfo(name: 'KoPubWorldDotum', weight: 300, file: 'KoPubWorld Dotum Light.ttf'),
  FontInfo(name: 'KoPubWorldDotum', weight: 500, file: 'KoPubWorld Dotum Medium.ttf'),
  FontInfo(name: 'KoPubWorldDotum', weight: 700, file: 'KoPubWorld Dotum Bold.ttf'),
  FontInfo(name: 'NanumBarunGothic', weight: 200, file: 'NanumBarunGothicUltraLight.ttf'),
  FontInfo(name: 'NanumBarunGothic', weight: 300, file: 'NanumBarunGothicLight.ttf'),
  FontInfo(name: 'NanumBarunGothic', weight: 400, file: 'NanumBarunGothic.ttf'),
  FontInfo(name: 'NanumBarunGothic', weight: 700, file: 'NanumBarunGothicBold.ttf'),
  FontInfo(name: 'RIDIBatang', weight: 400, file: 'RIDIBatang-Regular.ttf'),
  FontInfo(name: 'KoPubWorldBatang', weight: 300, file: 'KoPubWorld Batang Light.ttf'),
  FontInfo(name: 'KoPubWorldBatang', weight: 500, file: 'KoPubWorld Batang Medium.ttf'),
  FontInfo(name: 'KoPubWorldBatang', weight: 700, file: 'KoPubWorld Batang Bold.ttf'),
  FontInfo(name: 'NanumMyeongjo', weight: 400, file: 'NanumMyeongjo.ttf'),
  FontInfo(name: 'NanumMyeongjo', weight: 700, file: 'NanumMyeongjoBold.ttf'),
  FontInfo(name: 'NanumMyeongjo', weight: 800, file: 'NanumMyeongjoExtraBold.ttf'),
];

const _fontCdnBase = 'https://cdn.typie.net/fonts/editor';
const _emojiFontUrl = 'https://cdn.typie.net/fonts/editor/NotoColorEmoji.ttf';

enum WritingSystem { latin, korean, japanese, chinese }

class _FallbackFontConfig {
  const _FallbackFontConfig({required this.family, required this.weight, required this.url});

  final String family;
  final int weight;
  final String url;
}

const _writingSystemFontMap = <WritingSystem, List<_FallbackFontConfig>>{
  WritingSystem.latin: [],
  WritingSystem.korean: [],
  WritingSystem.japanese: [
    _FallbackFontConfig(
      family: 'Noto Sans JP',
      weight: 400,
      url: 'https://cdn.typie.net/fonts/fallback/NotoSansJP-Regular.ttf',
    ),
    _FallbackFontConfig(
      family: 'Noto Sans JP',
      weight: 700,
      url: 'https://cdn.typie.net/fonts/fallback/NotoSansJP-Bold.ttf',
    ),
  ],
  WritingSystem.chinese: [
    _FallbackFontConfig(
      family: 'Noto Sans SC',
      weight: 400,
      url: 'https://cdn.typie.net/fonts/fallback/NotoSansSC-Regular.ttf',
    ),
    _FallbackFontConfig(
      family: 'Noto Sans SC',
      weight: 700,
      url: 'https://cdn.typie.net/fonts/fallback/NotoSansSC-Bold.ttf',
    ),
  ],
};

class EditorFontManager {
  EditorFontManager(this._app);

  final NativeEditorApplication _app;
  final _loadedFonts = <String>{};
  final _loadedSystems = <WritingSystem>{};
  final _fetchingPromises = <String, Future<Uint8List>>{};
  bool pendingFontLoad = false;

  Future<Uint8List> _fetchFontData(String url) async {
    final existingFuture = _fetchingPromises[url];
    if (existingFuture != null) {
      return existingFuture;
    }

    final future = () async {
      final response = await Dio().get<List<int>>(url, options: Options(responseType: ResponseType.bytes));
      final data = response.data!;
      return Uint8List.fromList(data);
    }();

    _fetchingPromises[url] = future;

    try {
      return await future;
    } finally {
      unawaited(_fetchingPromises.remove(url));
    }
  }

  Future<void> loadFont(String name, int weight) async {
    final key = '$name-$weight';

    if (_loadedFonts.contains(key)) {
      return;
    }

    final fontInfo = editorFonts.where((f) => f.name == name && f.weight == weight).firstOrNull;
    if (fontInfo == null) {
      return;
    }

    try {
      final url = '$_fontCdnBase/${fontInfo.file}';
      final data = await _fetchFontData(url);

      if (_loadedFonts.contains(key)) {
        return;
      }

      _app.registerFont(name, weight, data);
      _loadedFonts.add(key);
    } catch (err) {
      // pass
    }
  }

  Future<void> loadInitialFonts() async {
    await loadFont('Pretendard', 400);
  }

  Future<void> loadPhantomFallback(Uint8List data) async {
    const key = 'Noto-Phantom-400';

    if (_loadedFonts.contains(key)) {
      return;
    }

    _app.registerFallbackFont('Noto-Phantom', 400, data);
    _loadedFonts.add(key);
  }

  Future<void> loadEmojiFallback() async {
    const key = 'NotoColorEmoji-400';

    if (_loadedFonts.contains(key)) {
      return;
    }

    try {
      final data = await _fetchFontData(_emojiFontUrl);
      if (_loadedFonts.contains(key)) {
        return;
      }

      _app.registerFallbackFont('NotoColorEmoji', 400, data);
      _loadedFonts.add(key);
    } catch (err) {
      // pass
    }
  }

  Future<bool> ensureRequiredFonts(List<(String, int)> fonts) async {
    final fontsToLoad = fonts.where((font) => !_loadedFonts.contains('${font.$1}-${font.$2}')).toList();
    if (fontsToLoad.isEmpty) {
      return false;
    }

    await Future.wait(fontsToLoad.map((font) => loadFont(font.$1, font.$2)));
    return true;
  }

  Future<bool> ensureRequiredScripts(List<WritingSystem> systems) async {
    final systemsToLoad = systems
        .where((s) => !_loadedSystems.contains(s) && (_writingSystemFontMap[s]?.isNotEmpty ?? false))
        .toList();
    if (systemsToLoad.isEmpty) {
      return false;
    }

    for (final system in systemsToLoad) {
      final fonts = _writingSystemFontMap[system] ?? [];
      for (final font in fonts) {
        try {
          final data = await _fetchFontData(font.url);
          _app.registerFallbackFont(font.family, font.weight, data);
        } catch (err) {
          // pass
        }
      }
      _loadedSystems.add(system);
    }

    return true;
  }
}

Map<String, List<int>> getAvailableFontsMap() {
  final map = <String, List<int>>{};
  for (final font in editorFonts) {
    map.putIfAbsent(font.name, () => []).add(font.weight);
  }
  return map;
}
