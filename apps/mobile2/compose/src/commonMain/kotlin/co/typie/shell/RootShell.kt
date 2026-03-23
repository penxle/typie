package co.typie.shell

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalFocusManager
import co.typie.auth.AuthService
import co.typie.auth.AuthState
import co.typie.overlay.LoaderOverlay
import co.typie.overlay.ToastOverlay
import co.typie.route.AuthRoutes
import co.typie.route.MainRoutes
import co.typie.screen.splash.SplashScreen
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
  val authState by authService.state.collectAsState()
  val bottomSheetHost = remember { BottomSheetHostState() }

  val focusManager = LocalFocusManager.current

  CompositionLocalProvider(LocalBottomSheetHost provides bottomSheetHost) {
    Box(
      Modifier
        .fillMaxSize()
        .pointerInput(Unit) { detectTapGestures { focusManager.clearFocus() } },
    ) {
      Crossfade(
        authState,
        modifier = Modifier
          .background(AppTheme.colors.surfaceSubtle)
          .hazeSource(LocalHazeState.current),
      ) { state ->
        when (state) {
          is AuthState.Initializing -> SplashScreen()
          is AuthState.Authenticated -> MainShell { route -> MainRoutes(route) }
          else -> AuthShell { route -> AuthRoutes(route) }
        }
      }

      BottomSheetHost(bottomSheetHost)
      LoaderOverlay()
      ToastOverlay()
    }
  }
}
