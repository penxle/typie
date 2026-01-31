import 'package:flutter/services.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/fonts.dart';

Future<(NativeEditorApplication, EditorFontManager)> initApplication() async {
  final icuData = await rootBundle.load('assets/native/icu_data.postcard');

  final app = NativeEditorApplication()
    ..loadIcuData(icuData.buffer.asUint8List())
    ..setAvailableFonts(getAvailableFontsMap());

  final fontManager = EditorFontManager(app);

  await fontManager.ensurePhantomFonts();

  return (app, fontManager);
}
