package co.typie.auth.sso

actual class GoogleSingleSignOnProvider : SingleSignOnAdapter {
  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    throw UnsupportedOperationException("Google SSO is not supported on JVM")
  }
}
