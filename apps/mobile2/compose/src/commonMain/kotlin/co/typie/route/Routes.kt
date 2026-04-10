package co.typie.route

import androidx.compose.runtime.Composable
import co.typie.screen.settings.ai_settings.AiSettingsScreen
import co.typie.screen.settings.delete_user.DeleteUserScreen
import co.typie.screen.editor.editor.EditorScreen
import co.typie.screen.settings.editor_settings.EditorSettingsScreen
import co.typie.screen.settings.font_settings.FontSettingsScreen
import co.typie.screen.space.folder.FolderScreen
import co.typie.screen.home.home.HomeScreen
import co.typie.screen.home.home_search.HomeSearchScreen
import co.typie.screen.auth.login.LoginScreen
import co.typie.screen.more.more.MoreScreen
import co.typie.screen.space.notes.NotesScreen
import co.typie.screen.settings.oss_licenses.OssLicensesScreen
import co.typie.screen.settings.profile_settings.ProfileSettingsScreen
import co.typie.screen.settings.preset_settings.PresetSettingsScreen
import co.typie.screen.settings.security_settings.SecuritySettingsScreen
import co.typie.screen.settings.settings.SettingsScreen
import co.typie.screen.settings.social_accounts.SocialAccountsScreen
import co.typie.screen.space.space.SpaceScreen
import co.typie.screen.more.stats.StatsScreen
import co.typie.screen.subscription.cancel_plan.CancelPlanScreen
import co.typie.screen.subscription.current_plan.CurrentPlanScreen
import co.typie.screen.subscription.enroll_plan.EnrollPlanScreen
import co.typie.screen.subscription.referral.ReferralScreen
import co.typie.screen.space.space_settings.SpaceSettingsScreen
import co.typie.screen.space.trash.TrashScreen
import co.typie.screen.settings.update_email.UpdateEmailScreen
import co.typie.screen.settings.update_password.UpdatePasswordScreen
import co.typie.screen.settings.update_profile.UpdateProfileScreen
import co.typie.screen.settings.widget_settings.WidgetSettingsScreen
import co.typie.screen.settings.text_replacements.TextReplacementsScreen

@Composable
fun MainRoutes(route: Route) {
  when (route) {
    is Route.Home -> HomeScreen()
    is Route.HomeSearch -> HomeSearchScreen()
    is Route.Space -> SpaceScreen()
    is Route.Notes -> NotesScreen()
    is Route.More -> MoreScreen()
    is Route.Stats -> StatsScreen()
    is Route.CurrentPlan -> CurrentPlanScreen()
    is Route.EnrollPlan -> EnrollPlanScreen()
    is Route.CancelPlan -> CancelPlanScreen()
    is Route.Referral -> ReferralScreen()
    is Route.Settings -> SettingsScreen()
    is Route.OssLicenses -> OssLicensesScreen()
    is Route.FontSettings -> FontSettingsScreen()
    is Route.ProfileSettings -> ProfileSettingsScreen()
    is Route.SecuritySettings -> SecuritySettingsScreen()
    is Route.DeleteUser -> DeleteUserScreen()
    is Route.EditorSettings -> EditorSettingsScreen()
    is Route.PresetSettings -> PresetSettingsScreen()
    is Route.TextReplacements -> TextReplacementsScreen()
    is Route.WidgetSettings -> WidgetSettingsScreen()
    is Route.AiSettings -> AiSettingsScreen()
    is Route.SocialAccounts -> SocialAccountsScreen()
    is Route.UpdateEmail -> UpdateEmailScreen()
    is Route.UpdateProfile -> UpdateProfileScreen()
    is Route.UpdatePassword -> UpdatePasswordScreen()
    is Route.SpaceSettings -> SpaceSettingsScreen()
    is Route.Trash -> TrashScreen(entityId = route.entityId)
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
