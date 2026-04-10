package co.typie.auth.sso

import co.typie.platform.ActivityContext

expect class GoogleSingleSignOnProvider() : SingleSignOnAdapter {
  context(_: ActivityContext)
  override suspend fun authenticate(): SingleSignOnCredential
}
