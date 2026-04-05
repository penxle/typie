package co.typie.auth

import kotlinx.serialization.Serializable

@Serializable
data class AuthTokens(
  val sessionToken: String,
  val accessToken: String? = null,
)
