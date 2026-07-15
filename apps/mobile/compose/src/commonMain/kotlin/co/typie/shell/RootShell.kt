package co.typie.shell

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.FrameRateCategory
import androidx.compose.ui.Modifier
import androidx.compose.ui.preferredFrameRate
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import co.typie.domain.auth.AuthService
import co.typie.domain.auth.AuthState
import co.typie.domain.bootstrap.BootstrapService
import co.typie.domain.bootstrap.BootstrapState
import co.typie.domain.preflight.PreflightService
import co.typie.domain.preflight.PreflightState
import co.typie.domain.pushnotification.PushNotificationService
import co.typie.domain.pushnotification.PushNotificationToastEffect
import co.typie.editor.sync.ActiveDocumentEditingSessions
import co.typie.editor.sync.orphanSweeper
import co.typie.editor.sync.ws.SyncWs
import co.typie.platform.appLifecycleService
import co.typie.platform.connectivityService
import co.typie.route.AuthRoutes
import co.typie.route.MainRoutes
import co.typie.screen.system.maintenance.MaintenanceScreen
import co.typie.screen.system.splash.SplashScreen
import co.typie.screen.system.updaterequired.UpdateRequiredScreen
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.DialogOverlay
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.loader.Loader
import co.typie.ui.component.loader.LoaderOverlay
import co.typie.ui.component.loader.LocalLoader
import co.typie.ui.component.popover.LocalPopoverOverlayState
import co.typie.ui.component.popover.PopoverOverlay
import co.typie.ui.component.popover.PopoverOverlayState
import co.typie.ui.component.popover.popoverOutsideTapHost
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.Sheet
import co.typie.ui.component.sheet.SheetOverlay
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.Toast
import co.typie.ui.component.toast.ToastOverlay
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import dev.chrisbanes.haze.hazeSource
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch

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
  LaunchedEffect(Unit) { PushNotificationService.launch() }

  LaunchedEffect(Unit) { connectivityService.monitor() }

  LaunchedEffect(Unit) {
    connectivityService.restorationGeneration.drop(1).collect {
      ActiveDocumentEditingSessions.retrySyncAll()
      orphanSweeper.sweep()
    }
  }

  LaunchedEffect(Unit) {
    snapshotFlow { AuthService.state }
      .filter { it is AuthState.Authenticated }
      .collect {
        orphanSweeper.resetPermanentFailures()
        orphanSweeper.sweep()
      }
  }

  val lifecycleOwner = LocalLifecycleOwner.current
  val lifecycleScope = rememberCoroutineScope()
  DisposableEffect(lifecycleOwner) {
    appLifecycleService.update(
      foreground = lifecycleOwner.lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)
    )
    val observer = LifecycleEventObserver { _, event ->
      when (event) {
        Lifecycle.Event.ON_START -> {
          appLifecycleService.update(foreground = true)
          SyncWs.onAppForeground()
          lifecycleScope.launch { orphanSweeper.sweep() }
        }
        Lifecycle.Event.ON_STOP -> {
          appLifecycleService.update(foreground = false)
          lifecycleScope.launch { ActiveDocumentEditingSessions.flushSyncAll() }
        }
        else -> {}
      }
    }
    lifecycleOwner.lifecycle.addObserver(observer)
    onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
  }

  val toast = remember { Toast() }
  val loader = remember { Loader() }
  val sheet = remember { Sheet() }
  val dialog = remember { Dialog() }
  val popover = remember { PopoverOverlayState() }

  val screen =
    when {
      BootstrapService.state !is BootstrapState.Ready -> RootScreen.Splash
      PreflightService.state is PreflightState.UnderMaintenance -> RootScreen.Maintenance
      PreflightService.state is PreflightState.UpdateRequired -> RootScreen.UpdateRequired
      AuthService.state is AuthState.Unauthenticated -> RootScreen.Auth
      else -> RootScreen.Main
    }

  CompositionLocalProvider(
    LocalSheet provides sheet,
    LocalDialog provides dialog,
    LocalToast provides toast,
    LocalLoader provides loader,
    LocalPopoverOverlayState provides popover,
  ) {
    Box(
      Modifier.fillMaxSize()
        .preferredFrameRate(FrameRateCategory.High)
        .popoverOutsideTapHost(state = popover)
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

      SheetOverlay(sheet)
      PopoverOverlay(popover)
      DialogOverlay(dialog)
      LoaderOverlay()
      ToastOverlay()
      PushNotificationToastEffect()
    }
  }
}
