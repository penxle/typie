package co.typie.shell

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.overlay.Toast
import co.typie.route.Route
import co.typie.route.toastBottomInset
import org.koin.compose.koinInject

@Composable
fun AuthShell(content: @Composable (Route) -> Unit) {
  val navigator = remember { Navigator(Route.Login) }
  val toast = koinInject<Toast>()
  LaunchedEffect(navigator.current) {
    toast.bottomInset = navigator.current.toastBottomInset
  }
  NavigationStack(navigator = navigator, content = content)
}
