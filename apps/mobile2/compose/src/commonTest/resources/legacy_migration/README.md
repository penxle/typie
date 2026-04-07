# Legacy Migration Fixtures

These fixtures are generated from the legacy Flutter app code paths and are used by `apps/mobile2` migration tests.

Regenerate them from [generate_fixtures.dart](/Users/dol/dev/typie/apps/mobile/tool/legacy_migration/generate_fixtures.dart):

```bash
cd /Users/dol/dev/typie/apps/mobile
dart run tool/legacy_migration/generate_fixtures.dart
```

Generated files:

- `auth_box.hive`: encrypted Hive box containing `session_token = "fixture-session-token"`
- `preference_box.hive`: plain Hive box containing the approved migration whitelist fixture values
- `theme_box.hive`: plain Hive box containing `mode = "dark"`
- `metadata.json`: deterministic `hive_encryption_key` and expected decoded values for tests
