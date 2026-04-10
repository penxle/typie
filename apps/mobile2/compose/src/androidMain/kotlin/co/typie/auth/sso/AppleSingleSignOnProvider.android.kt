package co.typie.auth.sso

import co.typie.platform.ActivityContext

actual class AppleSingleSignOnProvider : SingleSignOnAdapter {
  context(_: ActivityContext)
  actual override suspend fun authenticate(): SingleSignOnCredential {
    throw UnsupportedOperationException("Apple SSO is not supported on Android")
  }
}
