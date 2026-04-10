package co.typie.route

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlin.time.Instant

sealed interface Route {
  data object Home : Route
  data object HomeSearch : Route
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
  data class Editor(val slug: String) : Route
  data object Login : Route
  data object Offline : Route
  data class Maintenance(val title: String, val message: String, val until: Instant?) : Route
  data class UpdateRequired(val storeUrl: String, val currentVersion: String, val requiredVersion: String) : Route
}

enum class RouteTransitionStyle { Slide, Fade }

fun Route.transitionStyleTo(route: Route): RouteTransitionStyle = when {
  (this is Route.Home && route is Route.HomeSearch) ||
    (this is Route.HomeSearch && route is Route.Home) -> RouteTransitionStyle.Fade

  else -> RouteTransitionStyle.Slide
}

val Route.toastBottomInset: Dp
  get() = when (this) {
    is Route.Home, is Route.Space, is Route.Folder, is Route.Notes, is Route.More -> 72.dp // nav bar가 있는 스크린
    is Route.DeleteUser -> 120.dp // 하단 버튼이 2개인 스크린
    is Route.UpdateEmail,
    is Route.UpdateProfile,
    is Route.UpdatePassword,
    is Route.SocialAccounts,
    is Route.Referral,
    is Route.EnrollPlan,
    is Route.FontSettings,
    is Route.SpaceSettings -> 64.dp // 하단 버튼이 있는 스크린
    else -> 0.dp // 그 외
  }
