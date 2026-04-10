package co.typie.service

import co.typie.storage.Prefs

object DeveloperPreferencesService {
  const val DEV_MODE_KEY = "dev_mode"
  const val DEFAULT_DEV_MODE = false

  var devMode: Boolean by Prefs(DEV_MODE_KEY, DEFAULT_DEV_MODE)
}
