import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:hive_ce_flutter/hive_flutter.dart';
import 'package:injectable/injectable.dart';

@singleton
class KV {
  KV._(this._key);

  final List<int> _key;

  @FactoryMethod(preResolve: true)
  static Future<KV> create(FlutterSecureStorage storage) async {
    await Hive.initFlutter();

    final serializedKey = await storage.read(key: 'hive_encryption_key');

    List<int> key;
    if (serializedKey == null) {
      key = Hive.generateSecureKey();
      await storage.write(key: 'hive_encryption_key', value: base64Encode(key));
    } else {
      key = base64Decode(serializedKey);
    }

    return KV._(key);
  }

  Future<Box<dynamic>> openBox(String name, {bool encrypted = false}) async {
    if (encrypted) {
      return Hive.openBox(name, encryptionCipher: HiveAesCipher(_key));
    } else {
      return Hive.openBox(name);
    }
  }
}
