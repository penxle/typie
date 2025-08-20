import 'dart:async';

import 'package:hive_ce/hive.dart';
import 'package:injectable/injectable.dart';
import 'package:typie/services/kv.dart';

@singleton
class Pref {
  Pref._(this._box);

  final Box<dynamic> _box;

  @FactoryMethod(preResolve: true)
  static Future<Pref> create(KV hive) async {
    final box = await hive.openBox('preference_box');
    return Pref._(box);
  }

  String get siteId => _box.get('site_id') as String;
  set siteId(String value) => _box.put('site_id', value);

  bool get devMode => _box.get('dev_mode', defaultValue: false) as bool;
  set devMode(bool value) => _box.put('dev_mode', value);

  bool get typewriterEnabled => _box.get('typewriter_enabled', defaultValue: false) as bool;
  set typewriterEnabled(bool value) => _box.put('typewriter_enabled', value);

  double get typewriterPosition => _box.get('typewriter_position', defaultValue: 0.5) as double;
  set typewriterPosition(double value) => _box.put('typewriter_position', value);

  bool get lineHighlightEnabled => _box.get('line_highlight_enabled', defaultValue: true) as bool;
  set lineHighlightEnabled(bool value) => _box.put('line_highlight_enabled', value);

  Map<String, double>? get characterCountFloatingPosition {
    final data = _box.get('character_count_floating_position');
    if (data == null) {
      return null;
    }

    return Map<String, double>.from(data as Map);
  }

  set characterCountFloatingPosition(Map<String, double>? value) {
    if (value == null) {
      unawaited(_box.delete('character_count_floating_position'));
    } else {
      unawaited(_box.put('character_count_floating_position', value));
    }
  }

  bool get characterCountFloatingEnabled => _box.get('character_count_floating_enabled', defaultValue: false) as bool;
  set characterCountFloatingEnabled(bool value) => _box.put('character_count_floating_enabled', value);

  bool get widgetAutoFadeEnabled => _box.get('widget_auto_fade_enabled', defaultValue: true) as bool;
  set widgetAutoFadeEnabled(bool value) => _box.put('widget_auto_fade_enabled', value);
}
