package co.typie.route

import androidx.compose.runtime.Composable
import co.typie.screen.auth.login.LoginScreen
import co.typie.screen.editor.editor.EditorScreen
import co.typie.screen.home.home.HomeScreen
import co.typie.screen.home.search.SearchScreen
import co.typie.screen.more.more.MoreScreen
import co.typie.screen.more.stats.StatsScreen
import co.typie.screen.settings.aisettings.AiSettingsScreen
import co.typie.screen.settings.deleteuser.DeleteUserScreen
import co.typie.screen.settings.editorsettings.EditorSettingsScreen
import co.typie.screen.settings.fontsettings.FontSettingsScreen
import co.typie.screen.settings.osslicenses.OssLicensesScreen
import co.typie.screen.settings.presetsettings.PresetSettingsScreen
import co.typie.screen.settings.profilesettings.ProfileSettingsScreen
import co.typie.screen.settings.securitysettings.SecuritySettingsScreen
import co.typie.screen.settings.settings.SettingsScreen
import co.typie.screen.settings.socialaccounts.SocialAccountsScreen
import co.typie.screen.settings.textreplacements.TextReplacementsScreen
import co.typie.screen.settings.updateemail.UpdateEmailScreen
import co.typie.screen.settings.updatepassword.UpdatePasswordScreen
import co.typie.screen.settings.updateprofile.UpdateProfileScreen
import co.typie.screen.settings.widgetsettings.WidgetSettingsScreen
import co.typie.screen.space.folder.FolderScreen
import co.typie.screen.space.notes.NotesScreen
import co.typie.screen.space.space.SpaceScreen
import co.typie.screen.space.spacesettings.SpaceSettingsScreen
import co.typie.screen.space.trash.TrashScreen
import co.typie.screen.subscription.cancelplan.CancelPlanScreen
import co.typie.screen.subscription.currentplan.CurrentPlanScreen
import co.typie.screen.subscription.enrollplan.EnrollPlanScreen
import co.typie.screen.subscription.referral.ReferralScreen

@Composable
fun MainRoutes(route: Route) {
  when (route) {
    is Route.Home -> HomeScreen()
    is Route.Search -> SearchScreen()
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
    is Route.Editor -> EditorScreen(slug = route.entityId)
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
