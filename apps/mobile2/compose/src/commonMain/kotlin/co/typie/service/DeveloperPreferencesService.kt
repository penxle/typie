package co.typie.service

import co.typie.storage.Prefs
import org.koin.core.annotation.Single

@Single
class DeveloperPreferencesService(prefs: Prefs) {
  var devMode: Boolean by prefs(DEV_MODE_KEY, DEFAULT_DEV_MODE)

  companion object {
    const val DEV_MODE_KEY = "dev_mode"
    const val DEFAULT_DEV_MODE = false
  }
}
