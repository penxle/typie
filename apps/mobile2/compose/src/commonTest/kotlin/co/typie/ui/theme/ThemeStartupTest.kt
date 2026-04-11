package co.typie.ui.theme

import co.typie.bootstrap.BootstrapState
import kotlin.test.Test
import kotlin.test.assertEquals

class ThemeStartupTest {
  @Test
  fun `theme startup mode defers persisted theme until startup is ready`() {
    assertEquals(
      ThemeMode.System,
      resolveThemeModeForStartup(
        startupState = BootstrapState.NotReady,
        persistedThemeMode = ThemeMode.Dark,
      ),
    )
    assertEquals(
      ThemeMode.Dark,
      resolveThemeModeForStartup(
        startupState = BootstrapState.Ready,
        persistedThemeMode = ThemeMode.Dark,
      ),
    )
  }
}
