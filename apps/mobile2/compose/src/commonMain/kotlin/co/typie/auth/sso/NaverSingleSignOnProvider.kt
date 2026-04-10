package co.typie.auth.sso

expect class NaverSingleSignOnProvider() : SingleSignOnAdapter {
  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential
}
