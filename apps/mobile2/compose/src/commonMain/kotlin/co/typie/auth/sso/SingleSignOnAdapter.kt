package co.typie.auth.sso

import co.typie.platform.ActivityContext

interface SingleSignOnAdapter {
  context(_: ActivityContext)
  suspend fun authenticate(): SingleSignOnCredential
}
