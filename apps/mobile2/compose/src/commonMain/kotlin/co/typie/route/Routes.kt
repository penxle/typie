package co.typie.route

import androidx.compose.runtime.Composable
import co.typie.screen.detail.DetailScreen
import co.typie.screen.ai_settings.AiSettingsScreen
import co.typie.screen.delete_user.DeleteUserScreen
import co.typie.screen.editor.EditorScreen
import co.typie.screen.editor_settings.EditorSettingsScreen
import co.typie.screen.font_settings.FontSettingsScreen
import co.typie.screen.folder.FolderScreen
import co.typie.screen.home.HomeScreen
import co.typie.screen.login.LoginScreen
import co.typie.screen.more.MoreScreen
import co.typie.screen.notes.NotesScreen
import co.typie.screen.oss_licenses.OssLicensesScreen
import co.typie.screen.profile_settings.ProfileSettingsScreen
import co.typie.screen.security_settings.SecuritySettingsScreen
import co.typie.screen.settings.SettingsScreen
import co.typie.screen.social_accounts.SocialAccountsScreen
import co.typie.screen.space.SpaceScreen
import co.typie.screen.stats.StatsScreen
import co.typie.screen.subscription.ReferralScreen
import co.typie.screen.space_settings.SpaceSettingsScreen
import co.typie.screen.update_email.UpdateEmailScreen
import co.typie.screen.update_password.UpdatePasswordScreen
import co.typie.screen.update_profile.UpdateProfileScreen
import co.typie.screen.widget_settings.WidgetSettingsScreen

@Composable
fun MainRoutes(route: Route) {
  when (route) {
    is Route.Home -> HomeScreen()
    is Route.Space -> SpaceScreen()
    is Route.Notes -> NotesScreen()
    is Route.More -> MoreScreen()
    is Route.Stats -> StatsScreen()
    is Route.Referral -> ReferralScreen()
    is Route.Settings -> SettingsScreen()
    is Route.OssLicenses -> OssLicensesScreen()
    is Route.FontSettings -> FontSettingsScreen()
    is Route.ProfileSettings -> ProfileSettingsScreen()
    is Route.SecuritySettings -> SecuritySettingsScreen()
    is Route.DeleteUser -> DeleteUserScreen()
    is Route.EditorSettings -> EditorSettingsScreen()
    is Route.WidgetSettings -> WidgetSettingsScreen()
    is Route.AiSettings -> AiSettingsScreen()
    is Route.SocialAccounts -> SocialAccountsScreen()
    is Route.UpdateEmail -> UpdateEmailScreen()
    is Route.UpdateProfile -> UpdateProfileScreen()
    is Route.UpdatePassword -> UpdatePasswordScreen()
    is Route.SpaceSettings -> SpaceSettingsScreen()
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
