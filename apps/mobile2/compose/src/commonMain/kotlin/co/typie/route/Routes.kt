package co.typie.route

import androidx.compose.runtime.Composable
import co.typie.screen.detail.DetailScreen
import co.typie.screen.home.HomeScreen
import co.typie.screen.login.LoginScreen
import co.typie.screen.login_with_email.LoginWithEmailScreen
import co.typie.screen.notes.NotesScreen
import co.typie.screen.profile.ProfileScreen
import co.typie.screen.space.SpaceScreen

@Composable
fun MainRoutes(route: Route) {
  when (route) {
    is Route.Home -> HomeScreen()
    is Route.Space -> SpaceScreen()
    is Route.Notes -> NotesScreen()
    is Route.Profile -> ProfileScreen()
    is Route.Detail -> DetailScreen(id = route.id)
    else -> {}
  }
}

@Composable
fun AuthRoutes(route: Route) {
  when (route) {
    is Route.Login -> LoginScreen()
    is Route.LoginWithEmail -> LoginWithEmailScreen()
    else -> {}
  }
}
