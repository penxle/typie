import 'dart:async';

import 'package:flutter/material.dart';
import 'package:hive_ce/hive.dart';
import 'package:injectable/injectable.dart';
import 'package:typie/services/kv.dart';

@singleton
class AppTheme extends ChangeNotifier {
  AppTheme._(this._box) {
    final themeMode = _box.get('mode') as String? ?? ThemeMode.system.name;
    _mode = ThemeMode.values.firstWhere((mode) => mode.name == themeMode);
  }

  late ThemeMode _mode;
  final Box<dynamic> _box;

  @FactoryMethod(preResolve: true)
  static Future<AppTheme> create(KV kv) async {
    final box = await kv.openBox('theme_box');
    return AppTheme._(box);
  }

  ThemeMode get mode => _mode;

  set mode(ThemeMode mode) {
    _mode = mode;
    unawaited(_box.put('mode', mode.name));
    notifyListeners();
  }
}
