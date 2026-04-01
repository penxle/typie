package co.typie.route

import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

sealed interface Route {
  data object Home : Route
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
  data object TextReplacements : Route
  data object WidgetSettings : Route
  data object AiSettings : Route
  data object SpaceSettings : Route
  data class Detail(val id: String) : Route
  data class Folder(val entityId: String) : Route
  data class Editor(val slug: String) : Route
  data object Login : Route
}

val Route.toastBottomInset: Dp
  get() = when (this) {
    is Route.Home, is Route.Space, is Route.Notes, is Route.More -> 72.dp // nav bar가 있는 스크린
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
