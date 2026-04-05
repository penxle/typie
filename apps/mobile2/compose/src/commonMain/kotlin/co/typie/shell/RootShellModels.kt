package co.typie.shell

import co.typie.auth.AuthState
import co.typie.bootstrap.BootstrapState
import kotlin.time.Instant

internal sealed interface RootShellDestination {
  data object Splash : RootShellDestination
  data object Auth : RootShellDestination
  data object Main : RootShellDestination
  data object Offline : RootShellDestination
  data class Maintenance(
    val title: String,
    val message: String,
    val until: Instant?,
  ) : RootShellDestination

  data class UpdateRequired(
    val storeUrl: String,
    val currentVersion: String,
    val requiredVersion: String,
  ) : RootShellDestination
}
internal data class RootShellTargetState(
  val destination: RootShellDestination,
  val sessionToken: String?,
)

internal fun resolveRootShellDestination(
  authState: AuthState,
  bootstrapState: BootstrapState,
): RootShellDestination {
  if (authState is AuthState.Initializing || bootstrapState is BootstrapState.Loading) {
    return RootShellDestination.Splash
  }

  return when (bootstrapState) {
    is BootstrapState.Maintenance -> RootShellDestination.Maintenance(
      title = bootstrapState.title,
      message = bootstrapState.message,
      until = bootstrapState.until,
    )

    is BootstrapState.UpdateRequired -> RootShellDestination.UpdateRequired(
      storeUrl = bootstrapState.storeUrl,
      currentVersion = bootstrapState.currentVersion,
      requiredVersion = bootstrapState.requiredVersion,
    )

    else -> when (authState) {
      is AuthState.Authenticated -> RootShellDestination.Main
      is AuthState.Offline -> RootShellDestination.Offline
      else -> RootShellDestination.Auth
    }
  }
}

internal fun rootShellTargetState(
  authState: AuthState,
  sessionToken: String?,
  bootstrapState: BootstrapState,
): RootShellTargetState {
  val destination = resolveRootShellDestination(authState, bootstrapState)

  return RootShellTargetState(
    destination = destination,
    sessionToken = sessionToken.takeIf { destination is RootShellDestination.Main },
  )
}
