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
}
