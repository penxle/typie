package co.typie.auth.sso

actual class KakaoSingleSignOnProvider : SingleSignOnAdapter {
  actual override suspend fun authenticate(ctx: Any?): SingleSignOnCredential {
    throw UnsupportedOperationException("Kakao SSO is not supported on JVM")
  }
}
