package co.typie.migration

import co.typie.auth.AuthTokens
import co.typie.service.DeveloperPreferencesService
import co.typie.service.EditorPreferencesService
import co.typie.storage.Prefs
import co.typie.storage.Vault
import co.typie.ui.theme.ThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlinx.coroutines.test.runTest

class LegacyMigrationCoordinatorTest {
  private val hiveReader = LegacyHiveBoxReader()

  @Test
  fun `source_missing continues without auth or prefs import`() = runTest {
    val prefs = createLegacyMigrationTestPrefs()
    val vault = createLegacyMigrationTestVault()
    val stateStore = LegacyMigrationStateStore(prefs)
    val coordinator =
      createCoordinator(
        prefs = prefs,
        vault = vault,
        stateStore = stateStore,
        platformSource =
          object : LegacyMigrationPlatformSource {
            override suspend fun load(): LegacyMigrationSource? = null
          },
      )

    val result = coordinator.runIfNeeded()

    assertEquals(LegacyMigrationSourceState.Missing, result.sourceState)
    assertEquals(LegacyMigrationStepResult.NotAttempted, result.authResult)
    assertEquals(LegacyMigrationStepResult.NotAttempted, result.prefsResult)
    assertNull(TestVaultSnapshot(vault).tokens)
  }

  @Test
  fun `auth import succeeding while prefs import fails`() = runTest {
    val prefs = createLegacyMigrationTestPrefs()
    val vault = createLegacyMigrationTestVault()
    val stateStore = LegacyMigrationStateStore(prefs)
    val encryptionKey = fixtureEncryptionKey()
    val coordinator =
      createCoordinator(
        prefs = prefs,
        vault = vault,
        stateStore = stateStore,
        platformSource =
          object : LegacyMigrationPlatformSource {
            override suspend fun load(): LegacyMigrationSource {
              return LegacyMigrationSource(
                authBox =
                  LegacyEncryptedHiveBoxSource(
                    bytes = loadLegacyMigrationFixture("auth_box.hive"),
                    keyCrc = calculateLegacyHiveKeyCrc(encryptionKey),
                    decryptor =
                      LegacyAuthPayloadDecryptor { payload ->
                        decryptLegacyHiveAesPayload(payload, encryptionKey)
                      },
                  ),
                preferenceBox = byteArrayOf(1, 2, 3),
                themeBox = loadLegacyMigrationFixture("theme_box.hive"),
              )
            }
          },
      )

    val result = coordinator.runIfNeeded()

    assertEquals(LegacyMigrationStepResult.Imported, result.authResult)
    assertEquals(LegacyMigrationStepResult.Failed, result.prefsResult)
    assertEquals("fixture-session-token", TestVaultSnapshot(vault).tokens?.sessionToken)
  }

  @Test
  fun `prefs import succeeding while auth import fails`() = runTest {
    val prefs = createLegacyMigrationTestPrefs()
    val vault = createLegacyMigrationTestVault()
    val stateStore = LegacyMigrationStateStore(prefs)
    val coordinator =
      createCoordinator(
        prefs = prefs,
        vault = vault,
        stateStore = stateStore,
        platformSource =
          object : LegacyMigrationPlatformSource {
            override suspend fun load(): LegacyMigrationSource {
              return LegacyMigrationSource(
                authBox =
                  LegacyEncryptedHiveBoxSource(
                    bytes = loadLegacyMigrationFixture("auth_box.hive"),
                    keyCrc = 0,
                    decryptor = LegacyAuthPayloadDecryptor { error("boom") },
                  ),
                preferenceBox = loadLegacyMigrationFixture("preference_box.hive"),
                themeBox = loadLegacyMigrationFixture("theme_box.hive"),
              )
            }
          },
      )

    val result = coordinator.runIfNeeded()
    val snapshot = TestPrefsSnapshot(prefs)

    assertEquals(LegacyMigrationStepResult.Failed, result.authResult)
    assertEquals(LegacyMigrationStepResult.Imported, result.prefsResult)
    assertNull(TestVaultSnapshot(vault).tokens)
    assertEquals("site_fixture", snapshot.siteId)
    assertEquals(ThemeMode.Dark, snapshot.themeMode)
  }

  @Test
  fun `existing KMP auth and prefs cause import skip instead of overwrite`() = runTest {
    val prefs = createLegacyMigrationTestPrefs()
    val vault = createLegacyMigrationTestVault()
    val stateStore = LegacyMigrationStateStore(prefs)
    val existingTokens =
      TestVaultSnapshot(vault).apply {
        tokens = AuthTokens(sessionToken = "existing-session", accessToken = "existing-access")
      }
    TestPrefsSnapshot(prefs).apply {
      siteId = "existing-site"
      devMode = true
      typewriterEnabled = true
      typewriterPosition = 0.25
      lineHighlightEnabled = false
      autoSurroundEnabled = false
      characterCountFloatingEnabled = true
      widgetAutoFadeEnabled = false
      themeMode = ThemeMode.Light
    }
    val encryptionKey = fixtureEncryptionKey()
    val coordinator =
      createCoordinator(
        prefs = prefs,
        vault = vault,
        stateStore = stateStore,
        platformSource =
          object : LegacyMigrationPlatformSource {
            override suspend fun load(): LegacyMigrationSource {
              return LegacyMigrationSource(
                authBox =
                  LegacyEncryptedHiveBoxSource(
                    bytes = loadLegacyMigrationFixture("auth_box.hive"),
                    keyCrc = calculateLegacyHiveKeyCrc(encryptionKey),
                    decryptor =
                      LegacyAuthPayloadDecryptor { payload ->
                        decryptLegacyHiveAesPayload(payload, encryptionKey)
                      },
                  ),
                preferenceBox = loadLegacyMigrationFixture("preference_box.hive"),
                themeBox = loadLegacyMigrationFixture("theme_box.hive"),
              )
            }
          },
      )

    val result = coordinator.runIfNeeded()
    val snapshot = TestPrefsSnapshot(prefs)

    assertEquals(LegacyMigrationStepResult.Skipped, result.authResult)
    assertEquals(LegacyMigrationStepResult.Skipped, result.prefsResult)
    assertEquals("existing-session", existingTokens.tokens?.sessionToken)
    assertEquals("existing-site", snapshot.siteId)
    assertEquals(ThemeMode.Light, snapshot.themeMode)
  }

  private fun createCoordinator(
    prefs: Prefs,
    vault: Vault,
    stateStore: LegacyMigrationStateStore,
    platformSource: LegacyMigrationPlatformSource,
  ): LegacyMigrationCoordinator {
    return LegacyMigrationCoordinator(
      platformSource = platformSource,
      hiveBoxReader = hiveReader,
      stateStore = stateStore,
      authImporter = LegacyAuthImporter(vault, stateStore),
      prefsImporter = LegacyPrefsImporter(prefs, stateStore),
    )
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

  private class TestPrefsSnapshot(prefs: Prefs) {
    var siteId: String by prefs("site_id", "")
    var devMode: Boolean by
      prefs(DeveloperPreferencesService.DEV_MODE_KEY, DeveloperPreferencesService.DEFAULT_DEV_MODE)
    var typewriterEnabled: Boolean by
      prefs(
        EditorPreferencesService.TYPEWRITER_ENABLED_KEY,
        EditorPreferencesService.DEFAULT_TYPEWRITER_ENABLED,
      )
    var typewriterPosition: Double by
      prefs(
        EditorPreferencesService.TYPEWRITER_POSITION_KEY,
        EditorPreferencesService.DEFAULT_TYPEWRITER_POSITION,
      )
    var lineHighlightEnabled: Boolean by
      prefs(
        EditorPreferencesService.LINE_HIGHLIGHT_ENABLED_KEY,
        EditorPreferencesService.DEFAULT_LINE_HIGHLIGHT_ENABLED,
      )
    var autoSurroundEnabled: Boolean by
      prefs(
        EditorPreferencesService.AUTO_SURROUND_ENABLED_KEY,
        EditorPreferencesService.DEFAULT_AUTO_SURROUND_ENABLED,
      )
    var characterCountFloatingEnabled: Boolean by
      prefs(
        EditorPreferencesService.CHARACTER_COUNT_FLOATING_ENABLED_KEY,
        EditorPreferencesService.DEFAULT_CHARACTER_COUNT_FLOATING_ENABLED,
      )
    var widgetAutoFadeEnabled: Boolean by
      prefs(
        EditorPreferencesService.WIDGET_AUTO_FADE_ENABLED_KEY,
        EditorPreferencesService.DEFAULT_WIDGET_AUTO_FADE_ENABLED,
      )
    var themeMode: ThemeMode by prefs("theme_mode", ThemeMode.System)
  }

  private class TestVaultSnapshot(vault: Vault) {
    var tokens: AuthTokens? by vault("tokens", null)
  }

  @Test
  fun `auth import is not retried after session is cleared`() = runTest {
    val prefs = createLegacyMigrationTestPrefs()
    val vault = createLegacyMigrationTestVault()
    val stateStore = LegacyMigrationStateStore(prefs)
    val encryptionKey = fixtureEncryptionKey()
    val coordinator =
      createCoordinator(
        prefs = prefs,
        vault = vault,
        stateStore = stateStore,
        platformSource =
          object : LegacyMigrationPlatformSource {
            override suspend fun load(): LegacyMigrationSource {
              return LegacyMigrationSource(
                authBox =
                  LegacyEncryptedHiveBoxSource(
                    bytes = loadLegacyMigrationFixture("auth_box.hive"),
                    keyCrc = calculateLegacyHiveKeyCrc(encryptionKey),
                    decryptor =
                      LegacyAuthPayloadDecryptor { payload ->
                        decryptLegacyHiveAesPayload(payload, encryptionKey)
                      },
                  )
              )
            }
          },
      )
    val vaultSnapshot = TestVaultSnapshot(vault)

    val firstResult = coordinator.runIfNeeded()
    vaultSnapshot.tokens = null
    val secondResult = coordinator.runIfNeeded()

    assertEquals(LegacyMigrationStepResult.Imported, firstResult.authResult)
    assertEquals(LegacyMigrationStepResult.Skipped, secondResult.authResult)
    assertNull(vaultSnapshot.tokens)
  }
}
