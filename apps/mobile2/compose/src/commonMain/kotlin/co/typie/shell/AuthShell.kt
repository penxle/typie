package co.typie.shell

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import co.typie.navigation.NavigationScaffold
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.overlay.Toast
import co.typie.route.Route
import co.typie.route.toastBottomInset
import co.typie.ui.component.topbar.TopBarState
import org.koin.compose.koinInject

@Composable
fun AuthShell(content: @Composable (Route) -> Unit) {
  val navigator = remember { Navigator(Route.Login) }
  val topBarState = remember { TopBarState() }

  val toast = koinInject<Toast>()
  LaunchedEffect(navigator.current) {
    toast.bottomInset = navigator.current.toastBottomInset
  }

  NavigationScaffold(
    navigator = navigator,
    topBarState = topBarState,
  ) {
    NavigationStack(
      navigator = navigator,
      topBarState = topBarState,
      content = content,
    )
  }
}
