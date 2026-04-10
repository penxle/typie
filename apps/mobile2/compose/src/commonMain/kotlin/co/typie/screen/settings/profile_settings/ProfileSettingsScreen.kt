package co.typie.screen.settings.profile_settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.touchlab.kermit.Logger
import co.typie.datetime.formatKoreanDate
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.overlay.LocalToast
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.platform.Clipboard
import co.typie.result.onException
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.ui.component.AlertModal
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardRow
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.SettingSwitch
import co.typie.ui.component.SettingControlRow
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.time.Clock
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel

internal fun profileSettingsMarketingConsentMessage(marketingConsent: Boolean): String {
  val action = if (marketingConsent) "동의" else "거부"
  return "${Clock.System.now().formatKoreanDate()}에 ${action}처리되었어요."
}

@Composable
fun ProfileSettingsScreen() {
  val nav = Nav.current
  val model = koinViewModel<ProfileSettingsViewModel>()
  val toast = LocalToast.current
  val clipboard = koinInject<Clipboard>()
  val scrollState = rememberScrollState()
  val initialMarketingConsent = model.query.data.me.marketingConsent
  var marketingConsent by remember(initialMarketingConsent) { mutableStateOf(initialMarketingConsent) }
  var committedMarketingConsent by remember(initialMarketingConsent) { mutableStateOf(initialMarketingConsent) }
  var isUpdatingMarketingConsent by remember { mutableStateOf(false) }
  var marketingConsentModalMessage by remember { mutableStateOf<String?>(null) }
  var pendingMarketingConsent by remember { mutableStateOf<Boolean?>(null) }

  LaunchedEffect(pendingMarketingConsent) {
    val requested = pendingMarketingConsent ?: return@LaunchedEffect

    model.updateMarketingConsent(requested)
      .withDefaultExceptionHandler(toast)
      .onOk {
        committedMarketingConsent = requested
        marketingConsentModalMessage = profileSettingsMarketingConsentMessage(requested)
      }
      .onException {
        marketingConsent = committedMarketingConsent
      }

    isUpdatingMarketingConsent = false
    pendingMarketingConsent = null
  }

  suspend fun copyAccountId() {
    val accountId = model.query.data.me.id
    val copied = clipboard.copy(accountId, "text/plain")
    if (copied) {
      toast.show(ToastType.Success, "계정 ID가 복사되었어요.")
    } else {
      toast.show(ToastType.Error, "계정 ID를 복사할 수 없어요.")
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("프로필", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    scrollState = scrollState,
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
      Text(
        "프로필",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      SectionTitle("프로필")

      CardSurface(
        modifier = Modifier.fillMaxWidth(),
      ) {
        Column {
          CardRow(
            onClick = {
              nav.navigate(Route.UpdateProfile)
            },
          ) {
            Text(
              "프로필 변경",
              style = AppTheme.typography.label,
            )
          }

          CardDivider()

          CardRow(
            onClick = {
              nav.navigate(Route.UpdateEmail)
            },
          ) {
            Text(
              "이메일 변경",
              style = AppTheme.typography.label,
            )
          }
        }
      }

      SectionTitle("알림")

      CardSurface(
        modifier = Modifier.fillMaxWidth(),
      ) {
        SettingControlRow(
          label = "마케팅 수신",
          description = "새로운 기능과 이벤트 소식을 받아요.",
          enabled = !isUpdatingMarketingConsent,
          onClick = null,
          trailing = {
            SettingSwitch(
              checked = marketingConsent,
              enabled = !isUpdatingMarketingConsent,
              onCheckedChange = { next ->
                marketingConsent = next
                isUpdatingMarketingConsent = true
                pendingMarketingConsent = next
              },
            )
          },
        )
      }

      SectionTitle("지원")

      CardSurface(
        modifier = Modifier.fillMaxWidth(),
      ) {
        CardRow(
          onClick = ::copyAccountId,
        ) {
          Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(4.dp),
          ) {
            Text(
              "계정 ID",
              style = AppTheme.typography.label,
            )
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

      Spacer(Modifier.height(72.dp))
  }

  marketingConsentModalMessage?.let { message ->
    AlertModal(
      title = "타이피 마케팅 수신 동의",
      message = message,
      onConfirm = { marketingConsentModalMessage = null },
      onDismiss = { marketingConsentModalMessage = null },
    )
  }
}
