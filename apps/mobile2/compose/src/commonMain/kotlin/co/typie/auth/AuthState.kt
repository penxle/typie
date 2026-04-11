package co.typie.auth

import kotlinx.serialization.Serializable

sealed interface AuthState {
  data object Authenticated : AuthState

  data object Unauthenticated : AuthState
}

@Serializable data class AuthTokens(val sessionToken: String, val accessToken: String)
