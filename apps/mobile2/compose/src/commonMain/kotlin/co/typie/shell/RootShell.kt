package co.typie.shell

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.FrameRateCategory
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.preferredFrameRate
import co.typie.auth.AuthService
import co.typie.bootstrap.BootstrapDevSandbox
import co.typie.bootstrap.BootstrapService
import co.typie.bootstrap.effectiveBootstrapState
import co.typie.overlay.Loader
import co.typie.overlay.LoaderOverlay
import co.typie.overlay.LocalLoader
import co.typie.overlay.LocalToast
import co.typie.overlay.Toast
import co.typie.overlay.ToastOverlay
import co.typie.route.AuthRoutes
import co.typie.route.MainRoutes
import co.typie.route.Route
import co.typie.screen.system.maintenance.MaintenanceScreen
import co.typie.screen.system.offline.OfflineScreen
import co.typie.screen.system.splash.SplashScreen
import co.typie.screen.system.update_required.UpdateRequiredScreen
import co.typie.startup.AppStartupService
import co.typie.ui.component.bottomsheet.BottomSheetHost
import co.typie.ui.component.bottomsheet.BottomSheetHostState
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.sheet.LocalSheetHost
import co.typie.ui.component.sheet.SheetHostState
import co.typie.ui.component.sheet.SheetOverlayHosts
import co.typie.ui.component.sheet.SheetOverlayPresenterState
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeSource

@Composable
fun RootShell() {
  val appStartupService = AppStartupService
  val authService = AuthService
  val bootstrapService = BootstrapService
  val bootstrapDevSandbox = BootstrapDevSandbox
  val startupState by appStartupService.state.collectAsState()
  val authState by authService.state.collectAsState()
  val bootstrapState by bootstrapService.state.collectAsState()
  val bootstrapScenario by bootstrapDevSandbox.scenario.collectAsState()

  LaunchedEffect(Unit) {
    appStartupService.start()
  }

  val shellTargetState = rootShellTargetState(
    startupState = startupState,
    authState = authState,
    bootstrapState = effectiveBootstrapState(
      remoteState = bootstrapState,
      scenario = bootstrapScenario,
    ),
  )
  val toast = remember { Toast() }
  val loader = remember { Loader() }
  val bottomSheetHost = remember { BottomSheetHostState() }
  val sheetOverlayPresenter = remember { SheetOverlayPresenterState() }
  val sheetHostScope = rememberCoroutineScope()
  val sheetHost = remember(sheetOverlayPresenter, sheetHostScope) { SheetHostState(sheetOverlayPresenter, sheetHostScope) }

  val focusManager = LocalFocusManager.current

  CompositionLocalProvider(
    LocalBottomSheetHost provides bottomSheetHost,
    LocalSheetHost provides sheetHost,
    LocalToast provides toast,
    LocalLoader provides loader,
  ) {
    Box(
      Modifier
        .fillMaxSize()
        .preferredFrameRate(FrameRateCategory.High)
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
            is RootShellDestination.System -> when (val route = destination.route) {
              is Route.Offline -> OfflineScreen(
                onRetry = { authService.retryAsync() },
              )

              is Route.Maintenance -> MaintenanceScreen(
                title = route.title,
                message = route.message,
                until = route.until,
              )

              is Route.UpdateRequired -> UpdateRequiredScreen(
                storeUrl = route.storeUrl,
                currentVersion = route.currentVersion,
                requiredVersion = route.requiredVersion,
              )

              else -> {}
            }
          }
        }
      }

      BottomSheetHost(bottomSheetHost)
      SheetOverlayHosts(sheetOverlayPresenter)
      LoaderOverlay()
      ToastOverlay()
    }
  }
}
