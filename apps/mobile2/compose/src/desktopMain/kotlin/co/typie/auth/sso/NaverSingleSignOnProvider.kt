package co.typie.auth.sso

actual class NaverSingleSignOnProvider : SingleSignOnAdapter {
  actual override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    throw UnsupportedOperationException("Naver SSO is not supported on JVM")
  }
}
