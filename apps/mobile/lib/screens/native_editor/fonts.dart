import 'dart:async';
import 'dart:io';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/service.dart';

class FontInfo {
  const FontInfo({required this.family, required this.weight, required this.file});

  final String family;
  final int weight;
  final String file;
}

const _phantomFonts = <FontInfo>[
  FontInfo(family: 'Noto-Phantom-Emoji', weight: 400, file: 'Noto-Phantom-Emoji.woff2'),
  FontInfo(family: 'Noto-Phantom', weight: 400, file: 'Noto-Phantom.woff2'),
];

const defaultFonts = <FontInfo>[
  FontInfo(family: 'Pretendard', weight: 100, file: 'Pretendard-Thin.woff2'),
  FontInfo(family: 'Pretendard', weight: 200, file: 'Pretendard-ExtraLight.woff2'),
  FontInfo(family: 'Pretendard', weight: 300, file: 'Pretendard-Light.woff2'),
  FontInfo(family: 'Pretendard', weight: 400, file: 'Pretendard-Regular.woff2'),
  FontInfo(family: 'Pretendard', weight: 500, file: 'Pretendard-Medium.woff2'),
  FontInfo(family: 'Pretendard', weight: 600, file: 'Pretendard-SemiBold.woff2'),
  FontInfo(family: 'Pretendard', weight: 700, file: 'Pretendard-Bold.woff2'),
  FontInfo(family: 'Pretendard', weight: 800, file: 'Pretendard-ExtraBold.woff2'),
  FontInfo(family: 'Pretendard', weight: 900, file: 'Pretendard-Black.woff2'),
  FontInfo(family: 'KoPubWorldDotum', weight: 300, file: 'KoPubWorld Dotum Light.woff2'),
  FontInfo(family: 'KoPubWorldDotum', weight: 500, file: 'KoPubWorld Dotum Medium.woff2'),
  FontInfo(family: 'KoPubWorldDotum', weight: 700, file: 'KoPubWorld Dotum Bold.woff2'),
  FontInfo(family: 'NanumBarunGothic', weight: 200, file: 'NanumBarunGothicUltraLight.woff2'),
  FontInfo(family: 'NanumBarunGothic', weight: 300, file: 'NanumBarunGothicLight.woff2'),
  FontInfo(family: 'NanumBarunGothic', weight: 400, file: 'NanumBarunGothic.woff2'),
  FontInfo(family: 'NanumBarunGothic', weight: 700, file: 'NanumBarunGothicBold.woff2'),
  FontInfo(family: 'RIDIBatang', weight: 400, file: 'RIDIBatang-Regular.woff2'),
  FontInfo(family: 'KoPubWorldBatang', weight: 300, file: 'KoPubWorld Batang Light.woff2'),
  FontInfo(family: 'KoPubWorldBatang', weight: 500, file: 'KoPubWorld Batang Medium.woff2'),
  FontInfo(family: 'KoPubWorldBatang', weight: 700, file: 'KoPubWorld Batang Bold.woff2'),
  FontInfo(family: 'NanumMyeongjo', weight: 400, file: 'NanumMyeongjo.woff2'),
  FontInfo(family: 'NanumMyeongjo', weight: 700, file: 'NanumMyeongjoBold.woff2'),
  FontInfo(family: 'NanumMyeongjo', weight: 800, file: 'NanumMyeongjoExtraBold.woff2'),
];

enum WritingSystem { latin, korean, japanese, chinese, emoji }

const _fallbackFonts = <WritingSystem, List<FontInfo>>{
  WritingSystem.latin: [FontInfo(family: 'Pretendard', weight: 400, file: 'Pretendard-Regular.woff2')],
  WritingSystem.korean: [FontInfo(family: 'Pretendard', weight: 400, file: 'Pretendard-Regular.woff2')],
  WritingSystem.japanese: [
    FontInfo(family: 'Noto Sans JP', weight: 400, file: 'NotoSansJP-Regular.woff2'),
    FontInfo(family: 'Noto Sans JP', weight: 700, file: 'NotoSansJP-Bold.woff2'),
  ],
  WritingSystem.chinese: [
    FontInfo(family: 'Noto Sans SC', weight: 400, file: 'NotoSansSC-Regular.woff2'),
    FontInfo(family: 'Noto Sans SC', weight: 700, file: 'NotoSansSC-Bold.woff2'),
  ],
  WritingSystem.emoji: [FontInfo(family: 'NotoColorEmoji', weight: 400, file: 'NotoColorEmoji.woff2')],
};

const _fontCdnBase = 'https://cdn.typie.net/fonts/editor';
const _fontCacheDir = 'fonts';

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

typedef NetworkCallbacks = ({void Function()? onStart, void Function()? onEnd});

class EditorFontManager {
  EditorFontManager(this._app);

  final NativeEditorApplication _app;
  final _loadedFonts = <String>{};
  final _loadingFonts = <String, Future<void>>{};
  bool pendingFontLoad = false;

  Future<Uint8List> _fetchFont(String fileName, {NetworkCallbacks? callbacks}) async {
    final basePath = await _getCacheBasePath();
    final cacheFile = File('$basePath/$fileName');

    if (cacheFile.existsSync()) {
      return cacheFile.readAsBytes();
    }

    callbacks?.onStart?.call();
    try {
      final response = await serviceLocator<Dio>().get<List<int>>(
        '$_fontCdnBase/$fileName',
        options: Options(responseType: ResponseType.bytes),
      );
      final data = Uint8List.fromList(response.data!);

      unawaited(cacheFile.writeAsBytes(data));

      return data;
    } finally {
      callbacks?.onEnd?.call();
    }
  }

  Future<void> _addFont(FontInfo font, {NetworkCallbacks? callbacks}) async {
    final key = '${font.family}-${font.weight}';
    if (_loadedFonts.contains(key)) {
      return;
    }

    final existing = _loadingFonts[key];
    if (existing != null) {
      await existing;
      return;
    }

    final future = () async {
      try {
        final data = await _fetchFont(font.file, callbacks: callbacks);
        if (_loadedFonts.contains(key)) {
          return;
        }
        _app.addFont(font.family, font.weight, data);
        _loadedFonts.add(key);
      } catch (_) {}
    }();

    _loadingFonts[key] = future;
    try {
      await future;
    } finally {
      unawaited(_loadingFonts.remove(key));
    }
  }

  Future<void> ensurePhantomFonts() async {
    await Future.wait(_phantomFonts.map(_addFont));
    for (final font in _phantomFonts) {
      _app.registerFallbackFont(font.family);
    }
  }

  Future<bool> ensureRequiredFonts(List<(String, int)> fonts, {NetworkCallbacks? callbacks}) async {
    final toLoad = fonts
        .where((font) => !_loadedFonts.contains('${font.$1}-${font.$2}'))
        .map((font) => defaultFonts.where((f) => f.family == font.$1 && f.weight == font.$2).firstOrNull)
        .whereType<FontInfo>()
        .toList();

    if (toLoad.isEmpty) {
      return false;
    }

    await Future.wait(toLoad.map((font) => _addFont(font, callbacks: callbacks)));
    return true;
  }

  Future<bool> ensureRequiredWritingSystems(List<WritingSystem> systems, {NetworkCallbacks? callbacks}) async {
    final toLoad = systems
        .expand((system) => _fallbackFonts[system] ?? <FontInfo>[])
        .where((font) => !_loadedFonts.contains('${font.family}-${font.weight}'))
        .toList();

    if (toLoad.isEmpty) {
      return false;
    }

    await Future.wait(toLoad.map((font) => _addFont(font, callbacks: callbacks)));

    toLoad.map((f) => f.family).toSet().forEach(_app.registerFallbackFont);

    return true;
  }
}

Map<String, List<int>> getAvailableFontsMap() {
  final map = <String, List<int>>{};
  for (final font in defaultFonts) {
    map.putIfAbsent(font.family, () => []).add(font.weight);
  }
  return map;
}
