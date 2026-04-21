package co.typie.domain.auth.sso

import co.typie.platform.ActivityContext

expect class NaverSingleSignOnProvider() : SingleSignOnAdapter {
  context(activity: ActivityContext)
  override suspend fun authenticate(): SingleSignOnCredential
}
