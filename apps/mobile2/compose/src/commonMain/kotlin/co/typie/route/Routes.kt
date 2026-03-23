package co.typie.route

import androidx.compose.runtime.Composable
import co.typie.screen.detail.DetailScreen
import co.typie.screen.editor.EditorScreen
import co.typie.screen.folder.FolderScreen
import co.typie.screen.home.HomeScreen
import co.typie.screen.login.LoginScreen
import co.typie.screen.notes.NotesScreen
import co.typie.screen.profile.ProfileScreen
import co.typie.screen.space.SpaceScreen
import co.typie.screen.stats.StatsScreen
import co.typie.screen.update_profile.UpdateProfileScreen

@Composable
fun MainRoutes(route: Route) {
  when (route) {
    is Route.Home -> HomeScreen()
    is Route.Space -> SpaceScreen()
    is Route.Notes -> NotesScreen()
    is Route.Profile -> ProfileScreen()
    is Route.Stats -> StatsScreen()
    is Route.UpdateProfile -> UpdateProfileScreen()
    is Route.Detail -> DetailScreen(id = route.id)
    is Route.Folder -> FolderScreen(entityId = route.entityId)
    is Route.Editor -> EditorScreen(slug = route.slug)
    else -> {}
  }
}

@Composable
fun AuthRoutes(route: Route) {
  when (route) {
    is Route.Login -> LoginScreen()
    else -> {}
  }
}
