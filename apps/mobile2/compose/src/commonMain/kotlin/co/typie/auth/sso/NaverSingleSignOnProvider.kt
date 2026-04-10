package co.typie.auth.sso

import co.typie.platform.ActivityContext

expect class NaverSingleSignOnProvider() : SingleSignOnAdapter {
  context(_: ActivityContext)
  override suspend fun authenticate(): SingleSignOnCredential
}
