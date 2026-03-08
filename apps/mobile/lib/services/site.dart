import 'package:flutter/foundation.dart';
import 'package:injectable/injectable.dart';
import 'package:typie/services/preference.dart';

@singleton
class Site extends ValueNotifier<String> {
  Site._(this._pref) : super('');

  final Pref _pref;

  @FactoryMethod(preResolve: true)
  static Future<Site> create(Pref pref) async {
    final site = Site._(pref);
    final storedSiteId = pref.siteId;
    if (storedSiteId != null) {
      site.value = storedSiteId;
    }
    return site;
  }

  String get siteId => value;

  void setSiteId(String id) {
    _pref.siteId = id;
    value = id;
  }

  void ensureValidSiteId(List<String> siteIds) {
    if (value.isEmpty || !siteIds.contains(value)) {
      setSiteId(siteIds.first);
    }
  }
}
