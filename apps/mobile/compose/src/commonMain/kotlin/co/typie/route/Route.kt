package co.typie.route

import kotlinx.serialization.Serializable

@Serializable
sealed interface Route {
  @Serializable data object Home : Route

  @Serializable data object Search : Route

  @Serializable data object Space : Route

  @Serializable data object Notes : Route

  @Serializable data object More : Route

  @Serializable data object Stats : Route

  @Serializable data object CurrentPlan : Route

  @Serializable data object EnrollPlan : Route

  @Serializable data object CancelPlan : Route

  @Serializable data object UpdateEmail : Route

  @Serializable data object UpdateProfile : Route

  @Serializable data object UpdatePassword : Route

  @Serializable data object SocialAccounts : Route

  @Serializable data object ProfileSettings : Route

  @Serializable data object SecuritySettings : Route

  @Serializable data object DeleteUser : Route

  @Serializable data object Referral : Route

  @Serializable data object Settings : Route

  @Serializable data object Feedback : Route

  @Serializable data object OssLicenses : Route

  @Serializable data object FontSettings : Route

  @Serializable data object EditorSettings : Route

  @Serializable data object PresetSettings : Route

  @Serializable data object TextReplacements : Route

  @Serializable data object WidgetSettings : Route

  @Serializable data object AiSettings : Route

  @Serializable data object SpaceSettings : Route

  @Serializable data object UserGoal : Route

  @Serializable data class EntityGoal(val entityId: String) : Route

  @Serializable data class Trash(val entityId: String? = null) : Route

  @Serializable data class Folder(val entityId: String) : Route

  @Serializable data class FolderDetails(val entityId: String) : Route

  @Serializable data class Editor(val entityId: String) : Route

  @Serializable data class Document(val entityId: String) : Route

  @Serializable data class DocumentBodySettings(val entityId: String) : Route

  @Serializable data object Onboarding : Route

  @Serializable data object Login : Route
}

enum class RouteTransitionStyle {
  Slide,
  VerticalSlide,
  Fade,
}

fun Route.transitionStyleTo(route: Route): RouteTransitionStyle =
  when {
    (this is Route.Home && route is Route.Search) ||
      (this is Route.Search && route is Route.Home) -> RouteTransitionStyle.Fade

    (this is Route.Document && route is Route.DocumentBodySettings) ||
      (this is Route.DocumentBodySettings && route is Route.Document) -> RouteTransitionStyle.Slide

    isVerticalSlidePresentation() || route.isVerticalSlidePresentation() ->
      RouteTransitionStyle.VerticalSlide

    else -> RouteTransitionStyle.Slide
  }

private fun Route.isVerticalSlidePresentation(): Boolean =
  this is Route.Document ||
    this is Route.FolderDetails ||
    this is Route.EntityGoal ||
    this is Route.UserGoal

val Route.keepAlive: Boolean
  get() =
    when (this) {
      is Route.Editor -> true
      else -> false
    }

val Route.popGestureDisabled: Boolean
  get() =
    when (this) {
      is Route.Onboarding -> true
      else -> false
    }
