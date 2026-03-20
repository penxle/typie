package co.typie.auth.sso

interface SingleSignOnAdapter {
  suspend fun authenticate(ctx: Any? = null): SingleSignOnCredential
}
