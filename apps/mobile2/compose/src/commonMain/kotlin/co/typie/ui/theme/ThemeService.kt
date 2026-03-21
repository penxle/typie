package co.typie.ui.theme

import co.typie.storage.Prefs
import org.koin.core.annotation.Single

@Single
class ThemeService(prefs: Prefs) {
  var themeMode: ThemeMode by prefs(ThemeMode.System, "theme_mode")
}
