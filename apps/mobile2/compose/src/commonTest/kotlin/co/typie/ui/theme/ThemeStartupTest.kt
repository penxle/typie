package co.typie.ui.theme

import co.typie.migration.LegacyMigrationRunResult
import co.typie.migration.LegacyMigrationSourceState
import co.typie.migration.LegacyMigrationStepResult
import co.typie.startup.AppStartupState
import kotlin.test.Test
import kotlin.test.assertEquals

class ThemeStartupTest {
  @Test
  fun `theme startup mode defers persisted theme until startup is ready`() {
    assertEquals(
      ThemeMode.System,
      resolveThemeModeForStartup(
        startupState = AppStartupState.Migrating,
        persistedThemeMode = ThemeMode.Dark,
      ),
    )
    assertEquals(
      ThemeMode.Dark,
      resolveThemeModeForStartup(
        startupState = AppStartupState.Ready(readyMigrationResult()),
        persistedThemeMode = ThemeMode.Dark,
      ),
    )
  }

  private fun readyMigrationResult(): LegacyMigrationRunResult {
    return LegacyMigrationRunResult(
      sourceState = LegacyMigrationSourceState.Available,
      authResult = LegacyMigrationStepResult.Imported,
      prefsResult = LegacyMigrationStepResult.Imported,
    )
  }
}
