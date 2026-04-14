package co.typie.domain.auth.sso

import co.typie.platform.ActivityContext

actual class GoogleSingleSignOnProvider : SingleSignOnAdapter {
  context(activity: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
    throw UnsupportedOperationException("Google SSO is not supported on JVM")
  }
}
