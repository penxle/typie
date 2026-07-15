package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
import co.typie.route.Route

val Nav = staticCompositionLocalOf<Navigator> { error("No Navigator provided") }

val LocalRoute = staticCompositionLocalOf<Route> { error("No Route provided") }

internal val LocalNavigationPopNestedScroll =
  staticCompositionLocalOf<NavigationPopNestedScroll?> { null }

@Composable
internal fun Modifier.navigationPopNestedScroll(): Modifier {
  val navigationPopNestedScroll = LocalNavigationPopNestedScroll.current ?: return this
  return nestedScroll(navigationPopNestedScroll)
}
