package co.typie.ui.theme

import co.typie.storage.prefs

object ThemeService {
  var themeMode: ThemeMode by prefs("theme_mode", ThemeMode.System)
}
