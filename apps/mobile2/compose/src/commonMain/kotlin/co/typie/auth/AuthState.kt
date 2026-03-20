package co.typie.auth

sealed interface AuthState {
  data object Initializing : AuthState
  data object Authenticated : AuthState
  data object Unauthenticated : AuthState
  data object Offline : AuthState
}
