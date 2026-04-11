package co.typie.migration

import co.typie.auth.AuthTokens
import co.typie.storage.Preference
import co.typie.storage.Vault
import co.typie.ui.theme.ThemeMode
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class LegacyMigrationCoordinatorTest {
  @BeforeTest
  fun resetState() {
    Preference.legacySiteId.value = ""
    Preference.devMode.value = Preference.DEFAULT_DEV_MODE
    Preference.typewriterEnabled.value = Preference.DEFAULT_TYPEWRITER_ENABLED
    Preference.typewriterPosition.value = Preference.DEFAULT_TYPEWRITER_POSITION
    Preference.lineHighlightEnabled.value = Preference.DEFAULT_LINE_HIGHLIGHT_ENABLED
    Preference.autoSurroundEnabled.value = Preference.DEFAULT_AUTO_SURROUND_ENABLED
    Preference.characterCountFloatingEnabled.value =
      Preference.DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED
    Preference.widgetAutoFadeEnabled.value = Preference.DEFAULT_WIDGET_AUTO_FADE_ENABLED
    Preference.themeMode.value = ThemeMode.System
    Preference.migrationSchemaVersion.value = 0
    Preference.migrationLastResultName.value = LegacyMigrationPhaseStatus.NotStarted.name
    Preference.migrationLastAttemptAtMillis.value = 0L
    Preference.migrationCompletedAtMillis.value = 0L
    Preference.migrationHandledSession.value = false
    Preference.migrationImportedSession.value = false
    Preference.migrationImportedPrefs.value = false
    Preference.migrationImportedPrefKeys.value = emptyList()
    Preference.migrationSkippedPrefKeys.value = emptyList()
    Vault.authTokens.value = null
    Vault.legacyTokens.value = null
  }

  @Test
  fun `auth import stores session token in vault`() {
    val encryptionKey = fixtureEncryptionKey()
    val authValues =
      LegacyHiveBoxReader.readEncryptedBox(
        bytes = loadLegacyMigrationFixture("auth_box.hive"),
        keyCrc = calculateLegacyHiveKeyCrc(encryptionKey),
        decrypt = { payload -> decryptLegacyHiveAesPayload(payload, encryptionKey) },
      )
    val sessionToken = authValues["session_token"] as String

    val result = LegacyAuthImporter.importSessionToken(sessionToken)

    assertEquals(LegacyMigrationStepResult.Imported, result)
    assertEquals("fixture-session-token", Vault.legacyTokens.value?.sessionToken)
  }

  @Test
  fun `prefs import succeeding with fixture hive boxes`() {
    val preferenceValues =
      LegacyHiveBoxReader.readBox(loadLegacyMigrationFixture("preference_box.hive"))
    val themeValues = LegacyHiveBoxReader.readBox(loadLegacyMigrationFixture("theme_box.hive"))

    val report =
      LegacyPrefsImporter.import(
        LegacyPrefsImportSource(preferenceValues = preferenceValues, themeValues = themeValues)
      )

    assertTrue(report.importedKeys.isNotEmpty())
    assertEquals("site_fixture", Preference.legacySiteId.value)
    assertEquals(ThemeMode.Dark, Preference.themeMode.value)
  }

  @Test
  fun `existing KMP auth causes auth import skip`() {
    Vault.legacyTokens.value =
      AuthTokens(sessionToken = "existing-session", accessToken = "existing-access")

    val result = LegacyAuthImporter.importSessionToken("new-session-token")

    assertEquals(LegacyMigrationStepResult.Skipped, result)
    assertEquals("existing-session", Vault.legacyTokens.value?.sessionToken)
  }

  @Test
  fun `existing KMP prefs cause prefs import skip`() {
    Preference.legacySiteId.value = "existing-site"
    Preference.devMode.value = true
    Preference.typewriterEnabled.value = true
    Preference.typewriterPosition.value = 0.25
    Preference.lineHighlightEnabled.value = false
    Preference.autoSurroundEnabled.value = false
    Preference.characterCountFloatingEnabled.value = true
    Preference.widgetAutoFadeEnabled.value = false
    Preference.themeMode.value = ThemeMode.Light

    val preferenceValues =
      LegacyHiveBoxReader.readBox(loadLegacyMigrationFixture("preference_box.hive"))
    val themeValues = LegacyHiveBoxReader.readBox(loadLegacyMigrationFixture("theme_box.hive"))

    val report =
      LegacyPrefsImporter.import(
        LegacyPrefsImportSource(preferenceValues = preferenceValues, themeValues = themeValues)
      )

    assertEquals(emptyList(), report.importedKeys)
    assertEquals("existing-site", Preference.legacySiteId.value)
    assertEquals(ThemeMode.Light, Preference.themeMode.value)
  }

  @Test
  fun `auth import is not retried after session is handled`() {
    val encryptionKey = fixtureEncryptionKey()
    val authValues =
      LegacyHiveBoxReader.readEncryptedBox(
        bytes = loadLegacyMigrationFixture("auth_box.hive"),
        keyCrc = calculateLegacyHiveKeyCrc(encryptionKey),
        decrypt = { payload -> decryptLegacyHiveAesPayload(payload, encryptionKey) },
      )
    val sessionToken = authValues["session_token"] as String

    val firstResult = LegacyAuthImporter.importSessionToken(sessionToken)
    Vault.legacyTokens.value = null
    val secondResult = LegacyAuthImporter.importSessionToken(sessionToken)

    assertEquals(LegacyMigrationStepResult.Imported, firstResult)
    assertEquals(LegacyMigrationStepResult.Skipped, secondResult)
    assertNull(Vault.legacyTokens.value)
  }

  private fun fixtureEncryptionKey(): ByteArray {
    return byteArrayOf(
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
    )
  }
}
