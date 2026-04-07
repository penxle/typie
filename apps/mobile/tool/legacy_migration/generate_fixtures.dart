import 'dart:convert';
import 'dart:io';

import 'package:hive_ce/hive.dart';

const hiveEncryptionKey = <int>[
  0,
  1,
  2,
  3,
  4,
  5,
  6,
  7,
  8,
  9,
  10,
  11,
  12,
  13,
  14,
  15,
  16,
  17,
  18,
  19,
  20,
  21,
  22,
  23,
  24,
  25,
  26,
  27,
  28,
  29,
  30,
  31,
];

const authFixture = <String, Object>{'session_token': 'fixture-session-token'};

const preferenceFixture = <String, Object>{
  'site_id': 'site_fixture',
  'dev_mode': true,
  'typewriter_enabled': true,
  'typewriter_position': 0.25,
  'line_highlight_enabled': false,
  'auto_surround_enabled': false,
  'character_count_floating_enabled': true,
  'widget_auto_fade_enabled': false,
};

const themeFixture = <String, Object>{'mode': 'dark'};

Future<void> main() async {
  final scriptDir = Directory.fromUri(Platform.script.resolve('.'));
  final mobileDir = scriptDir.parent.parent;
  final outputDir = Directory.fromUri(
    mobileDir.uri.resolve('../mobile2/compose/src/commonTest/resources/legacy_migration/'),
  );
  final tempDir = await Directory.systemTemp.createTemp('typie-legacy-migration-fixtures-');

  await outputDir.create(recursive: true);

  try {
    Hive.init(tempDir.path);

    final authBox = await Hive.openBox<dynamic>('auth_box', encryptionCipher: HiveAesCipher(hiveEncryptionKey));
    await authBox.putAll(authFixture);
    await authBox.close();

    final preferenceBox = await Hive.openBox<dynamic>('preference_box');
    await preferenceBox.putAll(preferenceFixture);
    await preferenceBox.close();

    final themeBox = await Hive.openBox<dynamic>('theme_box');
    await themeBox.putAll(themeFixture);
    await themeBox.close();

    await Hive.close();

    await _copyBoxFile(tempDir, outputDir, 'auth_box');
    await _copyBoxFile(tempDir, outputDir, 'preference_box');
    await _copyBoxFile(tempDir, outputDir, 'theme_box');

    final metadataFile = File.fromUri(outputDir.uri.resolve('metadata.json'));
    await metadataFile.writeAsString(
      const JsonEncoder.withIndent('  ').convert({
        'hive_encryption_key_base64': 'AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=',
        'expected': {'auth_box': authFixture, 'preference_box': preferenceFixture, 'theme_box': themeFixture},
      }),
    );
  } finally {
    if (await tempDir.exists()) {
      await tempDir.delete(recursive: true);
    }
  }
}

Future<void> _copyBoxFile(Directory sourceDir, Directory outputDir, String boxName) async {
  final sourceFile = File('${sourceDir.path}/$boxName.hive');
  if (!await sourceFile.exists()) {
    throw StateError('Expected Hive file for $boxName at ${sourceFile.path}');
  }

  await sourceFile.copy('${outputDir.path}/$boxName.hive');
}
