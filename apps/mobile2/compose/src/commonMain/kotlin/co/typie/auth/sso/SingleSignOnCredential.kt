package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider

data class SingleSignOnCredential(
  val provider: SingleSignOnProvider,
  val params: Map<String, String>,
)
