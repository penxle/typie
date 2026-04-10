package co.typie.ui.theme

import co.typie.storage.Prefs

object ThemeService {
  var themeMode: ThemeMode by Prefs("theme_mode", ThemeMode.System)
}
