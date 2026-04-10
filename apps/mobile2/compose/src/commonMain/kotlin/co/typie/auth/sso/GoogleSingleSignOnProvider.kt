package co.typie.auth.sso

expect class GoogleSingleSignOnProvider() : SingleSignOnAdapter {
  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential
}
