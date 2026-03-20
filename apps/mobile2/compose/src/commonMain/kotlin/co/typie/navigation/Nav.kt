package co.typie.navigation

import androidx.compose.runtime.staticCompositionLocalOf

val Nav = staticCompositionLocalOf<Navigator> {
  error("No Navigator provided")
}
