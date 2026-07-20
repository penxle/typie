package co.typie.screen.settings.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.auth.AuthService
import co.typie.domain.settings.SettingSwitch
import co.typie.domain.settings.SettingsCardRow
import co.typie.domain.subscription.Entitlement
import co.typie.domain.subscription.SubscriptionService
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.navigation.Nav
import co.typie.platform.PlatformModule
import co.typie.route.Route
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetOptionList
import co.typie.ui.component.sheet.SheetOptionRow
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.ThemeMode

@Composable
fun SettingsScreen() {
  val model = viewModel { SettingsViewModel() }

  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current
  val sheet = LocalSheet.current
  val dialog = LocalDialog.current
  val uriHandler = LocalUriHandler.current

  val appVersion = remember {
    PlatformModule.deviceInfo.retrieve().let { "${it.appVersion} (${it.appBuildNumber})" }
  }
  var devModeTapCount by remember { mutableStateOf(0) }

  ProvideTopBar(
    center = { Text("설정", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text("설정", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

      SettingsSection("계정") {
        SettingsCardRow("프로필", onClick = { nav.navigate(Route.ProfileSettings) })
        CardDivider()
        SettingsCardRow("보안", onClick = { nav.navigate(Route.SecuritySettings) })
      }

      SettingsSection("스페이스") {
        SettingsCardRow("현재 스페이스 설정", onClick = { nav.navigate(Route.SpaceSettings) })
      }

      SettingsSection("환경") {
        SettingsCardRow(
          "테마",
          onClick = { sheet.present { ThemeModeSheet() } },
          trailing = {
            Text(
              text =
                when (Preference.themeMode) {
                  ThemeMode.System -> "시스템 설정"
                  ThemeMode.Light -> "라이트"
                  ThemeMode.Dark -> "다크"
                },
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textMuted,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )

            Icon(
              icon = Lucide.ChevronRight,
              modifier = Modifier.size(16.dp),
              tint = AppTheme.colors.textMuted,
            )
          },
        )
        CardDivider()
        SettingsCardRow("에디터", onClick = { nav.navigate(Route.EditorSettings) })
        CardDivider()
        SettingsCardRow("위젯", onClick = { nav.navigate(Route.WidgetSettings) })
        CardDivider()
        SettingsCardRow("폰트", onClick = { nav.navigate(Route.FontSettings) })
        CardDivider()
        SettingsCardRow("프리셋", onClick = { nav.navigate(Route.PresetSettings) })
        CardDivider()
        SettingsCardRow("텍스트 대치", onClick = { nav.navigate(Route.TextReplacements) })
      }

      SettingsSection("구독") {
        SettingsCardRow(
          "플랜",
          onClick = {
            when (SubscriptionService.entitlement) {
              is Entitlement.Active -> nav.navigate(Route.CurrentPlan)
              is Entitlement.Expired -> nav.navigate(Route.EnrollPlan)
              is Entitlement.Unknown -> {}
            }
          },
        )
        CardDivider()
        SettingsCardRow("초대", onClick = { nav.navigate(Route.Referral) })
      }

      SettingsSection("고급") { SettingsCardRow("AI", onClick = { nav.navigate(Route.AiSettings) }) }

      SettingsSection("서비스 정보") {
        SettingsCardRow(
          "이용약관",
          onClick = { uriHandler.openUri("https://typie.co/legal/terms") },
          trailing = {
            Icon(
              icon = Lucide.ExternalLink,
              modifier = Modifier.size(16.dp),
              tint = AppTheme.colors.textMuted,
            )
          },
        )
        CardDivider()
        SettingsCardRow(
          "개인정보처리방침",
          onClick = { uriHandler.openUri("https://typie.co/legal/privacy") },
          trailing = {
            Icon(
              icon = Lucide.ExternalLink,
              modifier = Modifier.size(16.dp),
              tint = AppTheme.colors.textMuted,
            )
          },
        )
        CardDivider()
        SettingsCardRow(
          "사업자 정보",
          onClick = {
            uriHandler.openUri("https://www.ftc.go.kr/bizCommPop.do?wrkr_no=6108803078")
          },
          trailing = {
            Icon(
              icon = Lucide.ExternalLink,
              modifier = Modifier.size(16.dp),
              tint = AppTheme.colors.textMuted,
            )
          },
        )
        CardDivider()
        SettingsCardRow("오픈소스 라이센스", onClick = { nav.navigate(Route.OssLicenses) })
        CardDivider()
        SettingsCardRow(
          "버전 정보",
          onClick = {
            if (Preference.devMode) {
              toast.success("이미 개발자입니다.")
            } else {
              devModeTapCount++
              when {
                devModeTapCount >= 7 -> {
                  devModeTapCount = 0
                  Preference.devMode = true
                  toast.success("개발자가 되셨습니다.")
                }
                devModeTapCount >= 4 -> {
                  toast.success("개발자가 되기까지 ${7 - devModeTapCount}번...")
                }
              }
            }
          },
          trailing = {
            Text(
              text = appVersion,
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textMuted,
              maxLines = 1,
              overflow = TextOverflow.Ellipsis,
            )
          },
        )
      }

      if (Preference.devMode) {
        SettingsSection("개발자") {
          SettingsCardRow(
            "개발자 모드",
            onClick = { Preference.devMode = !Preference.devMode },
            trailing = {
              SettingSwitch(
                checked = Preference.devMode,
                onCheckedChange = { next -> Preference.devMode = next },
              )
            },
          )
          CardDivider()
          SettingsCardRow("온보딩 미리보기", onClick = { nav.navigate(Route.Onboarding) })
          CardDivider()
          SettingsCardRow(
            "트라이얼 리마인더 초기화",
            onClick = {
              Preference.trialReminderLastShownDate = null
              toast.success("트라이얼 리마인더를 초기화했어요")
            },
          )
        }
      }

      SettingsSection("기타") {
        SettingsCardRow(
          "로그아웃",
          onClick = {
            val result =
              dialog.confirm(
                title = "로그아웃",
                message = "정말 로그아웃하시겠어요?",
                confirmText = "로그아웃",
                confirmIsDestructive = true,
              )

            if (result is DialogResult.Resolved) {
              AuthService.logout()
            }
          },
        )
      }
    }
  }
}

@Composable
private fun SettingsSection(title: String, content: @Composable ColumnScope.() -> Unit) {
  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    SectionTitle(title, modifier = Modifier.padding(top = 4.dp))
    CardSurface(modifier = Modifier.fillMaxWidth()) { Column(content = content) }
  }
}

@Composable
context(_: SheetScope<ThemeMode>)
private fun ThemeModeSheet() {
  SheetLayout(
    header = {
      SheetBar(
        center = {
          Text(
            text = "테마",
            style = AppTheme.typography.title,
            color = AppTheme.colors.textDefault,
            overflow = TextOverflow.Ellipsis,
            maxLines = 1,
          )
        }
      )
    }
  ) {
    SheetOptionList(items = ThemeMode.entries) { mode ->
      val (label, icon) =
        when (mode) {
          ThemeMode.System -> "시스템 설정" to Lucide.Smartphone
          ThemeMode.Light -> "라이트" to Lucide.Sun
          ThemeMode.Dark -> "다크" to Lucide.Moon
        }

      SheetOptionRow(
        selected = mode == Preference.themeMode,
        onClick = {
          Preference.themeMode = mode
          dismiss()
        },
      ) {
        Row(
          verticalAlignment = Alignment.CenterVertically,
          horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          Icon(icon = icon, modifier = Modifier.size(18.dp), tint = AppTheme.colors.textMuted)
          Text(
            text = label,
            style = AppTheme.typography.action,
            modifier = Modifier.weight(1f),
            color = AppTheme.colors.textDefault,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
        }
      }
    }
  }
}
