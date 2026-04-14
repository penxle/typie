package co.typie.screen.settings.profilesettings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.formatKoreanDate
import co.typie.domain.settings.SettingControlRow
import co.typie.domain.settings.SettingSwitch
import co.typie.domain.settings.SettingsCardRow
import co.typie.ext.verticalScroll
import co.typie.navigation.Nav
import co.typie.platform.PlatformModule
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.alert
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.time.Clock
import kotlinx.coroutines.launch

@Composable
fun ProfileSettingsScreen() {
  val model = viewModel { ProfileSettingsViewModel() }

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val nav = Nav.current
  val toast = LocalToast.current
  val dialog = LocalDialog.current

  fun updateMarketingConsent(consented: Boolean) {
    scope.launch {
      model.updateMarketingConsent(consented).withDefaultExceptionHandler(toast).onOk {
        dialog.alert(
          title = "타이피 마케팅 수신 동의",
          message =
            "${Clock.System.now().formatKoreanDate()}에 ${if (consented) "동의" else "거부"} 처리되었어요.",
        )
      }
    }
  }

  suspend fun copyAccountId() {
    val accountId = model.query.data.me.id
    val copied = PlatformModule.clipboard.copy(accountId, "text/plain")
    if (copied) {
      toast.success("계정 ID가 복사되었어요.")
    } else {
      toast.error("계정 ID를 복사할 수 없어요.")
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("프로필", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text("프로필", style = AppTheme.typography.display)

      SectionTitle("프로필")

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column {
          SettingsCardRow(label = "프로필 변경", onClick = { nav.navigate(Route.UpdateProfile) })

          CardDivider()

          SettingsCardRow(label = "이메일 변경", onClick = { nav.navigate(Route.UpdateEmail) })
        }
      }

      SectionTitle("알림")

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        SettingControlRow(
          label = "마케팅 수신",
          description = "새로운 기능과 이벤트 소식을 받아요.",
          onClick = { updateMarketingConsent(!model.query.data.me.marketingConsent) },
          trailing = {
            SettingSwitch(
              checked = model.query.data.me.marketingConsent,
              onCheckedChange = { next -> updateMarketingConsent(next) },
            )
          },
        )
      }

      SectionTitle("지원")

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        CardRow(onClick = { copyAccountId() }) {
          Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text("계정 ID", style = AppTheme.typography.label)
            Text(
              "문의나 지원 요청 시 이 ID를 알려주시면 더 빠르게 도와드릴 수 있어요.",
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textTertiary,
            )
          }

          Text(
            text = model.query.data.me.id,
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
          )
        }
      }
    }
  }
}
