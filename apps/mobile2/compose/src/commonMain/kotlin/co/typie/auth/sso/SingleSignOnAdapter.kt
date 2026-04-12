package co.typie.auth.sso

import co.typie.platform.ActivityContext

interface SingleSignOnAdapter {
  context(activity: ActivityContext)
  suspend fun authenticate(): SingleSignOnCredential
}
