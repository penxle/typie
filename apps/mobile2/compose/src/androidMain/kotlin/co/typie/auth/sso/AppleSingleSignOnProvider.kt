package co.typie.auth.sso

actual class AppleSingleSignOnProvider : SingleSignOnAdapter {
  actual override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    throw UnsupportedOperationException("Apple SSO is not supported on Android")
  }
}
