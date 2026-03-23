package co.typie.service

import co.typie.storage.Prefs
import org.koin.core.annotation.Single

@Single
class SiteService(prefs: Prefs) {
  var siteId: String by prefs("site_id", "")
}
