package co.typie.screen.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.route.Route
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.bottomsheet.BottomSheetScope
import co.typie.ui.component.bottomsheet.LocalBottomSheetHost
import co.typie.ui.component.bottomsheet.dismiss
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ThemeMode
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

data class SettingsItem(
  val label: String,
  val route: Route? = null,
  val action: SettingsItemAction? = null,
)

data class SettingsSection(
  val title: String,
  val items: List<SettingsItem>,
)

enum class SettingsItemAction {
  Theme,
}

data class SettingsThemeOption(
  val mode: ThemeMode,
  val label: String,
  val icon: IconData,
)

data class SettingsThemeSelectionItem(
  val mode: ThemeMode,
  val label: String,
  val icon: IconData,
  val selected: Boolean,
)

internal fun settingsRouteFor(item: SettingsItem): Route? {
  return item.route
}

internal fun settingsPasswordItemLabel(hasPassword: Boolean): String {
  return if (hasPassword) "비밀번호 변경" else "비밀번호 설정"
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

internal fun settingsThemeSelectionItems(selectedMode: ThemeMode): List<SettingsThemeSelectionItem> {
  return settingsThemeOptions().map { option ->
    SettingsThemeSelectionItem(
      mode = option.mode,
      label = option.label,
      icon = option.icon,
      selected = option.mode == selectedMode,
    )
  }
}

internal fun settingsSections(hasPassword: Boolean): List<SettingsSection> {
  return listOf(
    SettingsSection(
      title = "계정 설정",
      items = listOf(
        SettingsItem("이메일 변경", route = Route.UpdateEmail),
        SettingsItem("프로필 변경", route = Route.UpdateProfile),
        SettingsItem(settingsPasswordItemLabel(hasPassword), route = Route.UpdatePassword),
        SettingsItem("연결된 SNS 계정", route = Route.SocialAccounts),
      ),
    ),
    SettingsSection(
      title = "화면 설정",
      items = listOf(
        SettingsItem("테마", action = SettingsItemAction.Theme),
      ),
    ),
    SettingsSection(
      title = "편집 경험 설정",
      items = listOf(
        SettingsItem("에디터 설정"),
        SettingsItem("텍스트 대치"),
      ),
    ),
    SettingsSection(
      title = "스페이스",
      items = listOf(
        SettingsItem("현재 스페이스 설정"),
      ),
    ),
    SettingsSection(
      title = "이벤트 알림 설정",
      items = listOf(
        SettingsItem("이벤트 및 타이피 소식 받아보기"),
      ),
    ),
    SettingsSection(
      title = "서비스 정보",
      items = listOf(
        SettingsItem("이용약관"),
        SettingsItem("개인정보처리방침"),
        SettingsItem("사업자 정보"),
        SettingsItem("오픈소스 라이센스"),
        SettingsItem("버전 정보"),
      ),
    ),
    SettingsSection(
      title = "기타",
      items = listOf(
        SettingsItem("로그아웃"),
        SettingsItem("회원 탈퇴"),
      ),
    ),
  )
}

@Composable
fun SettingsScreen() {
  val nav = Nav.current
  val model = koinViewModel<SettingsViewModel>()
  val bottomSheetHost = LocalBottomSheetHost.current
  val toast = koinInject<Toast>()
  val scrollState = rememberScrollState()
  val themeModeState = LocalThemeMode.current
  val hasPassword = model.query.data.me.hasPassword
  val sections = remember(hasPassword) { settingsSections(hasPassword) }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("설정", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
  ) { contentPadding ->
    Column(
      modifier = Modifier
        .fillMaxSize()
        .verticalScroll(scrollState)
        .padding(contentPadding)
        .navigationBarsPadding(),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text(
        "설정",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      sections.forEach { section ->
        SettingsSectionCard(
          section = section,
          themeMode = themeModeState.value,
          onThemeClick = {
            bottomSheetHost.show {
              SettingsThemeSheet(
                themeMode = themeModeState.value,
                onThemeModeChange = { themeModeState.value = it },
              )
            }
          },
          onItemClick = { item ->
            val route = settingsRouteFor(item)

            if (route != null) {
              nav.navigate(route)
            } else {
              toast.show(ToastType.Notification, "준비 중인 기능이에요.")
            }
          },
        )
      }

      Spacer(Modifier.height(72.dp))
    }
  }
}

@Composable
private fun SettingsSectionCard(
  section: SettingsSection,
  themeMode: ThemeMode,
  onThemeClick: suspend () -> Unit,
  onItemClick: suspend (SettingsItem) -> Unit,
) {
  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    SectionTitle(
      section.title,
      modifier = Modifier.padding(top = 4.dp),
    )

    CardSurface(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column {
        section.items.forEachIndexed { index, item ->
          if (item.action == SettingsItemAction.Theme) {
            SettingsThemeRow(
              themeMode = themeMode,
              onClick = onThemeClick,
            )
          } else {
            SettingsRow(
              item = item,
              onClick = { onItemClick(item) },
            )
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
private fun SettingsRow(
  item: SettingsItem,
  onClick: suspend () -> Unit,
) {
  CardRow(
    onClick = onClick,
  ) {
    SettingsRowContent(
      label = item.label,
      trailing = {
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
private fun SettingsThemeRow(
  themeMode: ThemeMode,
  onClick: suspend () -> Unit,
) {
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
private fun BottomSheetScope<Unit>.SettingsThemeSheet(
  themeMode: ThemeMode,
  onThemeModeChange: (ThemeMode) -> Unit,
) {
  Column(
    modifier = Modifier
      .fillMaxWidth()
      .padding(horizontal = 16.dp),
    verticalArrangement = Arrangement.spacedBy(4.dp),
  ) {
    Text("테마", style = AppTheme.typography.title)

    settingsThemeSelectionItems(themeMode).forEach { item ->
      SettingsThemeSheetOption(
        item = item,
        onClick = {
          onThemeModeChange(item.mode)
          dismiss()
        },
      )
    }
  }
}

@Composable
private fun SettingsThemeSheetOption(
  item: SettingsThemeSelectionItem,
  onClick: suspend () -> Unit,
) {
  InteractionScope {
    Row(
      modifier = Modifier
        .fillMaxWidth()
        .clickable(onClick)
        .padding(vertical = 12.dp)
        .pressScale(),
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
      if (item.selected) {
        Icon(
          icon = Lucide.Check,
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.brand,
        )
      } else {
        Spacer(Modifier.size(16.dp))
      }
    }
  }
}

@Composable
private fun RowScope.SettingsRowContent(
  label: String,
  trailing: @Composable () -> Unit,
) {
  Text(
    text = label,
    style = AppTheme.typography.label,
    modifier = Modifier.weight(1f),
    maxLines = 1,
    overflow = TextOverflow.Ellipsis,
  )

  Row(
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(0.dp),
  ) { trailing() }
}
