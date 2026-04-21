package co.typie.domain.auth

import kotlinx.serialization.Serializable

sealed interface AuthState {
  data class Authenticated(val tokens: AuthTokens) : AuthState

  data object Unauthenticated : AuthState
}

@Serializable data class AuthTokens(val sessionToken: String, val accessToken: String)
