package co.typie.auth.sso

import co.typie.platform.ActivityContext

actual class KakaoSingleSignOnProvider : SingleSignOnAdapter {
  context(activity: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
    throw UnsupportedOperationException("Kakao SSO is not supported on JVM")
  }
}
