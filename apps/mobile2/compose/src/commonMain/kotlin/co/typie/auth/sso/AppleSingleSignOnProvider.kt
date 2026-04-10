package co.typie.auth.sso

expect class AppleSingleSignOnProvider() : SingleSignOnAdapter {
  override suspend fun authenticate(ctx: Any?): SingleSignOnCredential
}
