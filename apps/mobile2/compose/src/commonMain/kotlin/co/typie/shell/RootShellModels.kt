package co.typie.shell

import co.typie.auth.AuthState
import co.typie.bootstrap.BootstrapState
import co.typie.route.Route
import co.typie.startup.AppStartupState

internal sealed interface RootShellDestination {
  data object Splash : RootShellDestination

  data object Auth : RootShellDestination

  data object Main : RootShellDestination

  data class System(val route: Route) : RootShellDestination
}

internal data class RootShellTargetState(val destination: RootShellDestination)

internal fun resolveRootShellDestination(
  startupState: AppStartupState,
  authState: AuthState,
  bootstrapState: BootstrapState,
): RootShellDestination {
  if (startupState !is AppStartupState.Ready) {
    return RootShellDestination.Splash
  }

  if (bootstrapState is BootstrapState.Loading) {
    return RootShellDestination.Splash
  }

  return when (bootstrapState) {
    is BootstrapState.Maintenance ->
      RootShellDestination.System(
        Route.Maintenance(
          title = bootstrapState.title,
          message = bootstrapState.message,
          until = bootstrapState.until,
        )
      )

    is BootstrapState.UpdateRequired ->
      RootShellDestination.System(
        Route.UpdateRequired(
          storeUrl = bootstrapState.storeUrl,
          currentVersion = bootstrapState.currentVersion,
          requiredVersion = bootstrapState.requiredVersion,
        )
      )

    else ->
      when (authState) {
        is AuthState.Authenticated -> RootShellDestination.Main
        else -> RootShellDestination.Auth
      }
  }
}

internal fun rootShellTargetState(
  startupState: AppStartupState,
  authState: AuthState,
  bootstrapState: BootstrapState,
): RootShellTargetState {
  val destination = resolveRootShellDestination(startupState, authState, bootstrapState)

  return RootShellTargetState(destination = destination)
}
