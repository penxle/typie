import 'package:flutter/services.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/fonts.dart';

Future<(NativeEditorApplication, EditorFontManager)> initApplication() async {
  final (icuData, phantomFont) = await (
    rootBundle.load('assets/native/icu_data.postcard'),
    rootBundle.load('assets/native/Noto-Phantom.ttf'),
  ).wait;

  final app = NativeEditorApplication()
    ..loadIcuData(icuData.buffer.asUint8List())
    ..setAvailableFonts(getAvailableFontsMap());

  final fontManager = EditorFontManager(app);

  await fontManager.loadPhantomFallback(phantomFont.buffer.asUint8List());
  await Future.wait([fontManager.loadInitialFonts(), fontManager.loadEmojiFallback()]);

  return (app, fontManager);
}
