package co.typie.auth.sso

import co.typie.platform.ActivityContext

actual class NaverSingleSignOnProvider : SingleSignOnAdapter {
  context(_: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
    throw UnsupportedOperationException("Naver SSO is not supported on JVM")
  }
}
