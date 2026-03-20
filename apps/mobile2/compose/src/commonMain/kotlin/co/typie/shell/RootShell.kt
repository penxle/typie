package co.typie.shell

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import co.typie.auth.AuthState
import co.typie.auth.AuthService
import co.typie.route.AuthRoutes
import co.typie.route.MainRoutes
import co.typie.toast.ToastOverlay
import co.typie.ui.theme.AppTheme
import org.koin.compose.koinInject

@Composable
fun RootShell() {
  val authService = koinInject<AuthService>()
  val authState by authService.state.collectAsState()

  Box(Modifier.fillMaxSize()) {
    Crossfade(authState, modifier = Modifier.background(AppTheme.colors.surfaceDefault)) { state ->
      when (state) {
        is AuthState.Authenticated -> MainShell { route -> MainRoutes(route) }
        else -> AuthShell { route -> AuthRoutes(route) }
      }
    }
    ToastOverlay()
  }
}
