package co.typie.domain.auth.sso

import co.typie.platform.ActivityContext

interface SingleSignOnAdapter {
  context(activity: ActivityContext)
  suspend fun authenticate(): SingleSignOnCredential
}
