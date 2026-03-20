package co.typie.shell

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import co.typie.navigation.NavigationStack
import co.typie.navigation.Navigator
import co.typie.route.Route

@Composable
fun AuthShell(content: @Composable (Route) -> Unit) {
  val navigator = remember { Navigator(Route.Login) }
  NavigationStack(navigator = navigator, content = content)
}
