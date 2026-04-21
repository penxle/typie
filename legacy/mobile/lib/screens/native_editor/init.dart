import 'package:flutter/services.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/state/fonts.dart';
import 'package:typie/screens/native_editor/state/theme.dart';

NativeEditorApplication? _sharedApplication;
FontManager? _sharedFontManager;
Future<(NativeEditorApplication, FontManager)>? _initPromise;
List<Map<String, dynamic>>? _pendingTextReplacementRules;
Map<String, List<int>>? _pendingAvailableFonts;

Future<(NativeEditorApplication, FontManager)> getOrInitializeApplication() async {
  if (_sharedApplication != null && _sharedFontManager != null) {
    return (_sharedApplication!, _sharedFontManager!);
  }

  if (_initPromise != null) {
    return _initPromise!;
  }

  _initPromise = _initApplication();
  return _initPromise!;
}

void setTextReplacementRules(List<Map<String, dynamic>> rules) {
  _pendingTextReplacementRules = rules;
  _sharedApplication?.setTextReplacementRules(rules);
}

void setAvailableFonts(Map<String, List<int>> fonts) {
  _pendingAvailableFonts = fonts;
  _sharedApplication?.setAvailableFonts(fonts);
}

Future<(NativeEditorApplication, FontManager)> _initApplication() async {
  final icuData = await rootBundle.load('assets/native/icu.zst');

  final app = NativeEditorApplication()..loadIcuData(icuData.buffer.asUint8List());

  final fontManager = FontManager(app);

  await Future.wait([fontManager.initFonts(), initEditorTheme()]);

  if (_pendingTextReplacementRules != null) {
    app.setTextReplacementRules(_pendingTextReplacementRules!);
  }

  if (_pendingAvailableFonts != null) {
    app.setAvailableFonts(_pendingAvailableFonts!);
  }

  _sharedApplication = app;
  _sharedFontManager = fontManager;

  return (app, fontManager);
}
