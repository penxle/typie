package co.typie.shell

import co.typie.auth.AuthState

internal data class RootShellTargetState(
  val authState: AuthState,
  val sessionToken: String?,
)

internal fun rootShellTargetState(authState: AuthState, sessionToken: String?): RootShellTargetState {
  return RootShellTargetState(
    authState = authState,
    sessionToken = sessionToken.takeIf { authState is AuthState.Authenticated },
  )
}
