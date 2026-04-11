package co.typie.shell

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.FrameRateCategory
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.preferredFrameRate
import co.typie.auth.AuthService
import co.typie.auth.AuthState
import co.typie.bootstrap.BootstrapService
import co.typie.bootstrap.BootstrapState
import co.typie.overlay.Loader
import co.typie.overlay.LoaderOverlay
import co.typie.overlay.LocalLoader
import co.typie.overlay.LocalToast
import co.typie.overlay.Toast
import co.typie.overlay.ToastOverlay
import co.typie.preflight.PreflightService
import co.typie.preflight.PreflightState
import co.typie.route.AuthRoutes
import co.typie.route.MainRoutes
import co.typie.screen.system.maintenance.MaintenanceScreen
import co.typie.screen.system.splash.SplashScreen
import co.typie.screen.system.update_required.UpdateRequiredScreen
import co.typie.ui.component.sheet.LocalSheetHost
import co.typie.ui.component.sheet.SheetHostState
import co.typie.ui.component.sheet.SheetOverlayHosts
import co.typie.ui.component.sheet.SheetOverlayPresenterState
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeSource

private enum class RootScreen {
  Splash,
  Maintenance,
  UpdateRequired,
  Auth,
  Main,
}

@Composable
fun RootShell() {
  LaunchedEffect(Unit) { BootstrapService.launch() }

  val bootstrapState by BootstrapService.state.collectAsState()
  val preflightState by PreflightService.state.collectAsState()
  val authState by AuthService.state.collectAsState()

  val screen =
    when {
      bootstrapState !is BootstrapState.Ready -> RootScreen.Splash
      preflightState is PreflightState.UnderMaintenance -> RootScreen.Maintenance
      preflightState is PreflightState.UpdateRequired -> RootScreen.UpdateRequired
      authState is AuthState.Unauthenticated -> RootScreen.Auth
      else -> RootScreen.Main
    }

  val toast = remember { Toast() }
  val loader = remember { Loader() }
  val sheetOverlayPresenter = remember { SheetOverlayPresenterState() }
  val sheetHostScope = rememberCoroutineScope()
  val sheetHost =
    remember(sheetOverlayPresenter, sheetHostScope) {
      SheetHostState(sheetOverlayPresenter, sheetHostScope)
    }

  val focusManager = LocalFocusManager.current

  CompositionLocalProvider(
    LocalSheetHost provides sheetHost,
    LocalToast provides toast,
    LocalLoader provides loader,
  ) {
    Box(
      Modifier.fillMaxSize().preferredFrameRate(FrameRateCategory.High).pointerInput(Unit) {
        detectTapGestures { focusManager.clearFocus() }
      }
    ) {
      Crossfade(
        screen,
        modifier =
          Modifier.background(AppTheme.colors.surfaceDefault).hazeSource(LocalHazeState.current),
      ) { target ->
        when (target) {
          RootScreen.Splash -> SplashScreen()
          RootScreen.Maintenance -> MaintenanceScreen()
          RootScreen.UpdateRequired -> UpdateRequiredScreen()
          RootScreen.Auth -> AuthShell { route -> AuthRoutes(route) }
          RootScreen.Main -> MainShell { route -> MainRoutes(route) }
        }
      }

      SheetOverlayHosts(sheetOverlayPresenter)
      LoaderOverlay()
      ToastOverlay()
    }
  }
}
