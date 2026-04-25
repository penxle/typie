package co.typie.route

sealed interface Route {
  data object Home : Route

  data object Search : Route

  data object Space : Route

  data object Notes : Route

  data object More : Route

  data object Stats : Route

  data object CurrentPlan : Route

  data object EnrollPlan : Route

  data object CancelPlan : Route

  data object UpdateEmail : Route

  data object UpdateProfile : Route

  data object UpdatePassword : Route

  data object SocialAccounts : Route

  data object ProfileSettings : Route

  data object SecuritySettings : Route

  data object DeleteUser : Route

  data object Referral : Route

  data object Settings : Route

  data object Feedback : Route

  data object OssLicenses : Route

  data object FontSettings : Route

  data object EditorSettings : Route

  data object PresetSettings : Route

  data object TextReplacements : Route

  data object WidgetSettings : Route

  data object AiSettings : Route

  data object SpaceSettings : Route

  data class Trash(val entityId: String? = null) : Route

  data class Folder(val entityId: String) : Route

  data class FolderDetails(val entityId: String) : Route

  data class Editor(val entityId: String) : Route

  data class Document(val entityId: String) : Route

  data object Login : Route
}

enum class RouteTransitionStyle {
  Slide,
  Fade,
}

fun Route.transitionStyleTo(route: Route): RouteTransitionStyle =
  when {
    (this is Route.Home && route is Route.Search) ||
      (this is Route.Search && route is Route.Home) -> RouteTransitionStyle.Fade

    else -> RouteTransitionStyle.Slide
  }

val Route.keepAlive: Boolean
  get() =
    when (this) {
      is Route.Editor -> true
      else -> false
    }
