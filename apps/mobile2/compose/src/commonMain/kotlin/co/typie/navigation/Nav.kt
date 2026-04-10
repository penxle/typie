package co.typie.navigation

import androidx.compose.runtime.staticCompositionLocalOf
import co.typie.route.Route

val Nav = staticCompositionLocalOf<Navigator> { error("No Navigator provided") }

val LocalRoute = staticCompositionLocalOf<Route> { error("No Route provided") }
