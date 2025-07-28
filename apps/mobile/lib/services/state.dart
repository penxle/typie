import 'package:hive_ce/hive.dart';
import 'package:injectable/injectable.dart';
import 'package:typie/services/kv.dart';

@singleton
class AppState {
  AppState._(this._box);

  final Box<dynamic> _box;

  @FactoryMethod(preResolve: true)
  static Future<AppState> create(KV hive) async {
    final box = await hive.openBox('state_box');
    return AppState._(box);
  }

  String? getSerializedPostSelection(String slug) {
    return _box.get('post_selection_$slug') as String?;
  }

  Future<void> setSerializedPostSelection(String slug, String selection) async {
    await _box.put('post_selection_$slug', selection);
  }
}
