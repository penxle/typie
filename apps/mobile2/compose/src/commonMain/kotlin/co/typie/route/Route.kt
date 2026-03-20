package co.typie.route

sealed interface Route {
  data object Home : Route
  data object Space : Route
  data object Notes : Route
  data object Profile : Route
  data class Detail(val id: String) : Route
  data object Login : Route
  data object LoginWithEmail : Route
}
