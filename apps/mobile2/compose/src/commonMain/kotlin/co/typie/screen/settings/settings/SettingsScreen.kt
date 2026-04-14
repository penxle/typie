package co.typie.screen.settings.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.subscription.SubscriptionService
import co.typie.subscription.SubscriptionServiceState
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.SettingSwitch
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.dialog.error
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetOptionList
import co.typie.ui.component.sheet.SheetOptionRow
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ThemeMode
import kotlinx.coroutines.launch

data class SettingsItem(
  val label: String,
  val route: Route? = null,
  val action: SettingsItemAction? = null,
  val externalUrl: String? = null,
)

data class SettingsSection(val title: String, val items: List<SettingsItem>)

enum class SettingsItemAction {
  Theme,
  Plan,
  DeveloperMode,
  Logout,
  VersionInfo,
}

data class SettingsThemeOption(val mode: ThemeMode, val label: String, val icon: IconData)

data class SettingsThemeSelectionItem(
  val mode: ThemeMode,
  val label: String,
  val icon: IconData,
  val selected: Boolean,
)

data class SettingsVersionTapResult(
  val nextTapCount: Int,
  val enableDeveloperMode: Boolean,
  val message: String? = null,
)

private const val SETTINGS_DEVELOPER_MODE_REQUIRED_TAP_COUNT = 7
private const val SETTINGS_DEVELOPER_MODE_HINT_START_TAP_COUNT = 4

private val SettingsThemeSheetPadding =
  SheetPadding(header = PaddingValues(horizontal = 16.dp), body = PaddingValues(horizontal = 16.dp))

internal fun settingsRouteFor(item: SettingsItem): Route? {
  return item.route
}

internal fun settingsTrailingIcon(item: SettingsItem): IconData {
  return if (item.externalUrl != null) Lucide.ExternalLink else Lucide.ChevronRight
}

internal fun settingsThemeOptions(): List<SettingsThemeOption> {
  return listOf(
    SettingsThemeOption(mode = ThemeMode.System, label = "시스템 설정", icon = Lucide.Smartphone),
    SettingsThemeOption(mode = ThemeMode.Light, label = "라이트", icon = Lucide.Sun),
    SettingsThemeOption(mode = ThemeMode.Dark, label = "다크", icon = Lucide.Moon),
  )
}

internal fun settingsThemeModeLabel(mode: ThemeMode): String {
  return settingsThemeOptions().first { it.mode == mode }.label
}

internal fun settingsThemeSelectionItems(
  selectedMode: ThemeMode
): List<SettingsThemeSelectionItem> {
  return settingsThemeOptions().map { option ->
    SettingsThemeSelectionItem(
      mode = option.mode,
      label = option.label,
      icon = option.icon,
      selected = option.mode == selectedMode,
    )
  }
}

internal suspend fun confirmSettingsLogout(onDismiss: () -> Unit, onLogout: suspend () -> Unit) {
  onLogout()
  onDismiss()
}

internal fun settingsVersionTapResult(
  devModeEnabled: Boolean,
  tapCount: Int,
): SettingsVersionTapResult {
  if (devModeEnabled) {
    return SettingsVersionTapResult(
      nextTapCount = 0,
      enableDeveloperMode = false,
      message = "이미 개발자입니다.",
    )
  }

  val nextTapCount = tapCount + 1

  if (nextTapCount >= SETTINGS_DEVELOPER_MODE_REQUIRED_TAP_COUNT) {
    return SettingsVersionTapResult(
      nextTapCount = 0,
      enableDeveloperMode = true,
      message = "개발자가 되셨습니다.",
    )
  }

  if (nextTapCount >= SETTINGS_DEVELOPER_MODE_HINT_START_TAP_COUNT) {
    return SettingsVersionTapResult(
      nextTapCount = nextTapCount,
      enableDeveloperMode = false,
      message = "개발자가 되기까지 ${SETTINGS_DEVELOPER_MODE_REQUIRED_TAP_COUNT - nextTapCount}번...",
    )
  }

  return SettingsVersionTapResult(nextTapCount = nextTapCount, enableDeveloperMode = false)
}

internal fun settingsSections(devModeEnabled: Boolean = false): List<SettingsSection> {
  return buildList {
    add(
      SettingsSection(
        title = "계정",
        items =
          listOf(
            SettingsItem("프로필", route = Route.ProfileSettings),
            SettingsItem("보안", route = Route.SecuritySettings),
          ),
      )
    )
    add(
      SettingsSection(
        title = "스페이스",
        items =
          listOf(
            // TODO: 스페이스 설정 진입 트래킹
            SettingsItem("현재 스페이스 설정", route = Route.SpaceSettings)
          ),
      )
    )
    add(
      SettingsSection(
        title = "환경",
        items =
          listOf(
            SettingsItem("테마", action = SettingsItemAction.Theme),
            SettingsItem("에디터", route = Route.EditorSettings),
            SettingsItem("위젯", route = Route.WidgetSettings),
            SettingsItem("폰트", route = Route.FontSettings),
            SettingsItem("프리셋", route = Route.PresetSettings),
            SettingsItem("텍스트 대치", route = Route.TextReplacements),
          ),
      )
    )
    add(
      SettingsSection(
        title = "구독",
        items =
          listOf(
            SettingsItem("플랜", action = SettingsItemAction.Plan),
            SettingsItem("초대", route = Route.Referral),
          ),
      )
    )
    add(SettingsSection(title = "고급", items = listOf(SettingsItem("AI", route = Route.AiSettings))))
    add(
      SettingsSection(
        title = "서비스 정보",
        items =
          listOf(
            SettingsItem("이용약관", externalUrl = "https://typie.co/legal/terms"),
            SettingsItem("개인정보처리방침", externalUrl = "https://typie.co/legal/privacy"),
            SettingsItem(
              "사업자 정보",
              externalUrl = "https://www.ftc.go.kr/bizCommPop.do?wrkr_no=6108803078",
            ),
            SettingsItem("오픈소스 라이센스", route = Route.OssLicenses),
            SettingsItem("버전 정보", action = SettingsItemAction.VersionInfo),
          ),
      )
    )
    if (devModeEnabled) {
      add(
        SettingsSection(
          title = "개발자",
          items = listOf(SettingsItem("개발자 모드", action = SettingsItemAction.DeveloperMode)),
        )
      )
    }
    add(
      SettingsSection(
        title = "기타",
        items = listOf(SettingsItem("로그아웃", action = SettingsItemAction.Logout)),
      )
    )
  }
}

@Composable
fun SettingsScreen() {
  val nav = Nav.current
  val uriHandler = LocalUriHandler.current
  val model = viewModel { SettingsViewModel() }
  val dialog = LocalDialog.current
  val sheet = LocalSheet.current
  val scope = rememberCoroutineScope()
  val toast = LocalToast.current
  val authService = model.authService
  val deviceInfo = model.deviceInfo
  val scrollState = rememberScrollState()
  val themeModeState = LocalThemeMode.current
  val devModeEnabled = Preference.devMode
  val subscriptionState = SubscriptionService.state
  val sections = remember(devModeEnabled) { settingsSections(devModeEnabled = devModeEnabled) }
  var appVersion by remember { mutableStateOf<String?>(null) }
  var devModeTapCount by remember { mutableStateOf(0) }

  LaunchedEffect(model.query.state) {
    if (model.query.state is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { model.query.refetch() })
    }
  }

  LaunchedEffect(deviceInfo) {
    appVersion =
      runCatching { deviceInfo.retrieve().appVersion.trim().takeIf { it.isNotEmpty() } }.getOrNull()
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("설정", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(
    scrollState = scrollState,
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    Text("설정", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

    sections.forEach { section ->
      SettingsSectionCard(
        section = section,
        themeMode = themeModeState.value,
        appVersion = appVersion,
        devModeEnabled = devModeEnabled,
        onThemeClick = {
          scope.launch {
            val result = sheet.present { SettingsThemeContent(themeMode = themeModeState.value) }
            if (result != null) {
              // TODO: 테마 변경 트래킹
              Preference.themeMode = result
            }
          }
        },
        onVersionInfoClick = {
          val result =
            settingsVersionTapResult(devModeEnabled = devModeEnabled, tapCount = devModeTapCount)

          devModeTapCount = result.nextTapCount

          if (result.enableDeveloperMode) {
            Preference.devMode = true
          }

          result.message?.let { message -> toast.show(ToastType.Success, message) }
        },
        onDeveloperModeChange = { next -> Preference.devMode = next },
        onItemClick = { item ->
          val route = settingsRouteFor(item)

          if (route != null) {
            nav.navigate(route)
          } else if (item.externalUrl != null) {
            uriHandler.openUri(item.externalUrl)
          } else if (item.action == SettingsItemAction.Plan) {
            when (subscriptionState) {
              is SubscriptionServiceState.Subscribed -> nav.navigate(Route.CurrentPlan)
              is SubscriptionServiceState.NotSubscribed -> nav.navigate(Route.EnrollPlan)
              is SubscriptionServiceState.Unknown ->
                toast.show(ToastType.Notification, "이용권 상태를 확인 중이에요.")
            }
          } else if (item.action == SettingsItemAction.Logout) {
            val result =
              dialog.confirm(
                title = "로그아웃",
                message = "정말 로그아웃하시겠어요?",
                confirmText = "로그아웃",
                confirmIsDestructive = true,
              )
            if (result is DialogResult.Resolved) {
              authService.logout()
            }
          } else {
            toast.show(ToastType.Notification, "준비 중인 기능이에요.")
          }
        },
      )
    }

    Spacer(Modifier.size(72.dp))
  }
}

@Composable
private fun SettingsSectionCard(
  section: SettingsSection,
  themeMode: ThemeMode,
  appVersion: String?,
  devModeEnabled: Boolean,
  onThemeClick: suspend () -> Unit,
  onVersionInfoClick: suspend () -> Unit,
  onDeveloperModeChange: (Boolean) -> Unit,
  onItemClick: suspend (SettingsItem) -> Unit,
) {
  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    SectionTitle(section.title, modifier = Modifier.padding(top = 4.dp))

    CardSurface(modifier = Modifier.fillMaxWidth()) {
      Column {
        section.items.forEachIndexed { index, item ->
          if (item.action == SettingsItemAction.Theme) {
            SettingsThemeRow(themeMode = themeMode, onClick = onThemeClick)
          } else if (item.action == SettingsItemAction.VersionInfo) {
            SettingsVersionRow(appVersion = appVersion, onClick = onVersionInfoClick)
          } else if (item.action == SettingsItemAction.DeveloperMode) {
            SettingsDeveloperModeRow(
              enabled = devModeEnabled,
              onCheckedChange = onDeveloperModeChange,
            )
          } else {
            SettingsRow(item = item, onClick = { onItemClick(item) })
          }

          if (index < section.items.lastIndex) {
            CardDivider()
          }
        }
      }
    }
  }
}

@Composable
private fun SettingsVersionRow(appVersion: String?, onClick: suspend () -> Unit) {
  CardRow(onClick = onClick) {
    SettingsRowContent(
      label = "버전 정보",
      trailing = {
        Text(
          text = appVersion ?: "확인 중",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      },
    )
  }
}

@Composable
private fun SettingsDeveloperModeRow(enabled: Boolean, onCheckedChange: (Boolean) -> Unit) {
  CardRow(onClick = { onCheckedChange(!enabled) }) {
    SettingsRowContent(
      label = "개발자 모드",
      trailing = { SettingSwitch(checked = enabled, onCheckedChange = onCheckedChange) },
    )
  }
}

@Composable
private fun SettingsRow(item: SettingsItem, onClick: suspend () -> Unit) {
  CardRow(onClick = onClick) {
    SettingsRowContent(
      label = item.label,
      trailing = {
        Icon(
          icon = settingsTrailingIcon(item),
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.textTertiary,
        )
      },
    )
  }
}

@Composable
private fun SettingsThemeRow(themeMode: ThemeMode, onClick: suspend () -> Unit) {
  CardRow(onClick = onClick) {
    SettingsRowContent(
      label = "테마",
      trailing = {
        Text(
          text = settingsThemeModeLabel(themeMode),
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
        Spacer(Modifier.size(4.dp))
        Icon(
          icon = Lucide.ChevronRight,
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.textTertiary,
        )
      },
    )
  }
}

@Composable
context(_: SheetScope<ThemeMode>)
private fun SettingsThemeContent(themeMode: ThemeMode) {
  SheetLayout(
    padding = SettingsThemeSheetPadding,
    verticalSpacing = 8.dp,
    header = {
      SheetBar(
        center = {
          Text(
            text = "테마",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textPrimary,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    },
  ) {
    SheetOptionList(items = settingsThemeSelectionItems(themeMode)) { item ->
      SheetOptionRow(selected = item.selected, onClick = { complete(item.mode) }) {
        Row(
          verticalAlignment = Alignment.CenterVertically,
          horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          Icon(
            icon = item.icon,
            modifier = Modifier.size(18.dp),
            tint = AppTheme.colors.textSecondary,
          )
          Text(
            text = item.label,
            style = AppTheme.typography.action,
            modifier = Modifier.weight(1f),
            color = AppTheme.colors.textPrimary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
        }
      }
    }
  }
}

@Composable
context(rowScope: RowScope)
private fun SettingsRowContent(label: String, trailing: @Composable () -> Unit) {
  Text(
    text = label,
    style = AppTheme.typography.label,
    modifier = with(rowScope) { Modifier.weight(1f) },
    maxLines = 1,
    overflow = TextOverflow.Ellipsis,
  )

  Row(
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(0.dp),
  ) {
    trailing()
  }
}
