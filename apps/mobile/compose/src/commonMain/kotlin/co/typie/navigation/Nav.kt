package co.typie.navigation

import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.nestedScroll
import co.typie.route.Route

val Nav = staticCompositionLocalOf<Navigator> { error("No Navigator provided") }

val LocalRoute = staticCompositionLocalOf<Route> { error("No Route provided") }

internal val LocalNavigationPopNestedScrollConnection =
  staticCompositionLocalOf<NestedScrollConnection?> { null }

internal val LocalNavigationPopNestedScrollCancel = staticCompositionLocalOf<(() -> Unit)?> { null }

@Composable
internal fun Modifier.navigationPopNestedScroll(): Modifier {
  val connection = LocalNavigationPopNestedScrollConnection.current ?: return this
  return nestedScroll(connection)
}
