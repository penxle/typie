package co.typie.auth.sso

expect class KakaoSingleSignOnProvider() : SingleSignOnAdapter {
  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential
}
