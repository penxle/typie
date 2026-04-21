import 'dart:convert';

import 'package:flutter/services.dart';

int _colorToU32(String color, [int alpha = 0xff]) {
  final clean = color.replaceAll('#', '');
  if (clean.length < 6) {
    return 0x000000ff;
  }
  final r = int.parse(clean.substring(0, 2), radix: 16);
  final g = int.parse(clean.substring(2, 4), radix: 16);
  final b = int.parse(clean.substring(4, 6), radix: 16);
  return ((r << 24) | (g << 16) | (b << 8) | alpha) & 0xffffffff;
}

Map<String, String> _assembleVariant(Map<String, dynamic> data, String variant) {
  final shared = (data['shared'] as Map<String, dynamic>).cast<String, String>();
  final isLight = variant.startsWith('light-');
  final modeShared = (data[isLight ? 'lightShared' : 'darkShared'] as Map<String, dynamic>).cast<String, String>();
  final unique = ((data['variants'] as Map<String, dynamic>)[variant] as Map<String, dynamic>).cast<String, String>();
  return {...shared, ...modeShared, ...unique};
}

Map<String, int> _buildTheme(Map<String, String> rawColors) {
  return rawColors.map((k, v) => MapEntry(k, _colorToU32(v)));
}

late Map<String, String> _lightRawColors;
late Map<String, String> _darkRawColors;
late Map<String, int> lightTheme;
late Map<String, int> darkTheme;

Future<void> initEditorTheme() async {
  final raw = await rootBundle.loadString('assets/native/theme.json');
  final data = jsonDecode(raw) as Map<String, dynamic>;

  _lightRawColors = _assembleVariant(data, 'light-white');
  _darkRawColors = _assembleVariant(data, 'dark-black');
  lightTheme = _buildTheme(_lightRawColors);
  darkTheme = _buildTheme(_darkRawColors);
}

Map<String, int> getEditorTheme(Brightness brightness) {
  return brightness == Brightness.dark ? darkTheme : lightTheme;
}

Color getEditorColor(Brightness brightness, String key) {
  final hex = (brightness == Brightness.dark ? _darkRawColors : _lightRawColors)[key]!;
  final value = int.parse(hex.substring(1), radix: 16);
  return Color(0xFF000000 | value);
}
