package co.typie.auth.sso

import co.typie.platform.ActivityContext

expect class AppleSingleSignOnProvider() : SingleSignOnAdapter {
  context(activity: ActivityContext)
  override suspend fun authenticate(): SingleSignOnCredential
}
