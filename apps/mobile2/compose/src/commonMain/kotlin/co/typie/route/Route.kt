package co.typie.route

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

sealed interface Route {
  data object Home : Route
  data object Space : Route
  data object Notes : Route
  data object Profile : Route
  data object UpdateProfile : Route
  data class Detail(val id: String) : Route
  data object Login : Route
  data object LoginWithEmail : Route
}

val Route.toastBottomInset: Dp
  get() = when (this) {
    is Route.Home, is Route.Space, is Route.Notes, is Route.Profile -> 72.dp
    is Route.UpdateProfile, is Route.LoginWithEmail -> 64.dp
    else -> 0.dp
  }
