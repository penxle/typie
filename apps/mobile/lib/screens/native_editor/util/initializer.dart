import 'package:flutter/services.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/fonts.dart';

NativeEditorApplication? _sharedApplication;
EditorFontManager? _sharedFontManager;
Future<(NativeEditorApplication, EditorFontManager)>? _initPromise;

Future<(NativeEditorApplication, EditorFontManager)> getOrInitializeApplication() async {
  if (_sharedApplication != null && _sharedFontManager != null) {
    return (_sharedApplication!, _sharedFontManager!);
  }

  if (_initPromise != null) {
    return _initPromise!;
  }

  _initPromise = _initApplication();
  return _initPromise!;
}

Future<(NativeEditorApplication, EditorFontManager)> _initApplication() async {
  final icuData = await rootBundle.load('assets/native/icu_data.postcard');

  final app = NativeEditorApplication()
    ..loadIcuData(icuData.buffer.asUint8List())
    ..setAvailableFonts(getAvailableFontsMap());

  final fontManager = EditorFontManager(app);

  await fontManager.ensurePhantomFonts();

  _sharedApplication = app;
  _sharedFontManager = fontManager;

  return (app, fontManager);
}
