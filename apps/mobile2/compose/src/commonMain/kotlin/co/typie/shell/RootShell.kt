package co.typie.shell

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import co.typie.auth.AuthService
import co.typie.bootstrap.BootstrapDevSandbox
import co.typie.bootstrap.BootstrapService
import co.typie.bootstrap.effectiveBootstrapState
import co.typie.overlay.LoaderOverlay
import co.typie.overlay.ToastOverlay
import co.typie.route.AuthRoutes
import co.typie.route.MainRoutes
import co.typie.screen.app_state.MaintenanceScreen
import co.typie.screen.app_state.OfflineScreen
import co.typie.screen.splash.SplashScreen
import co.typie.screen.app_state.UpdateRequiredScreen
import co.typie.ui.component.bottomsheet.BottomSheetHost
import co.typie.ui.component.bottomsheet.BottomSheetHostState
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeSource
import org.koin.compose.koinInject

@Composable
fun RootShell() {
  val authService = koinInject<AuthService>()
  val bootstrapService = koinInject<BootstrapService>()
  val bootstrapDevSandbox = koinInject<BootstrapDevSandbox>()
  val authState by authService.state.collectAsState()
  val bootstrapState by bootstrapService.state.collectAsState()
  val bootstrapScenario by bootstrapDevSandbox.scenario.collectAsState()
  val shellTargetState = rootShellTargetState(
    authState = authState,
    sessionToken = authService.tokens?.sessionToken,
    bootstrapState = effectiveBootstrapState(
      remoteState = bootstrapState,
      scenario = bootstrapScenario,
    ),
  )
  val bottomSheetHost = remember { BottomSheetHostState() }

  val focusManager = LocalFocusManager.current

  CompositionLocalProvider(LocalBottomSheetHost provides bottomSheetHost) {
    Box(
      Modifier
        .fillMaxSize()
        .pointerInput(Unit) { detectTapGestures { focusManager.clearFocus() } },
    ) {
      Crossfade(
        shellTargetState,
        modifier = Modifier
          .background(AppTheme.colors.surfaceDefault)
          .hazeSource(LocalHazeState.current),
      ) { state ->
        key(state) {
          when (val destination = state.destination) {
            is RootShellDestination.Splash -> SplashScreen()
            is RootShellDestination.Main -> MainShell { route -> MainRoutes(route) }
            is RootShellDestination.Auth -> AuthShell { route -> AuthRoutes(route) }
            is RootShellDestination.Offline -> OfflineScreen {
              authService.retryAsync()
            }
            is RootShellDestination.Maintenance -> MaintenanceScreen(
              title = destination.title,
              message = destination.message,
              until = destination.until,
            )
            is RootShellDestination.UpdateRequired -> UpdateRequiredScreen(
              storeUrl = destination.storeUrl,
              currentVersion = destination.currentVersion,
              requiredVersion = destination.requiredVersion,
            )
          }
        }
      }

      BottomSheetHost(bottomSheetHost)
      LoaderOverlay()
      ToastOverlay()
    }
  }
}
