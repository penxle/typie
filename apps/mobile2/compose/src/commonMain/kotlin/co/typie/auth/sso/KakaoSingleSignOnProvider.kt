package co.typie.auth.sso

import co.typie.platform.ActivityContext

expect class KakaoSingleSignOnProvider() : SingleSignOnAdapter {
  context(activity: ActivityContext)
  override suspend fun authenticate(): SingleSignOnCredential
}
