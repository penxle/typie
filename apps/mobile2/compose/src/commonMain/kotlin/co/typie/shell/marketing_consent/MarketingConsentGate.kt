package co.typie.shell.marketing_consent

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.datetime.formatKoreanDate
import co.typie.graphql.MarketingConsentGate_Query
import co.typie.graphql.MarketingConsentGate_UpdateMarketingConsent_Mutation
import co.typie.graphql.QueryState
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UpdateMarketingConsentInput
import co.typie.graphql.watchQuery
import co.typie.icons.Lucide
import co.typie.overlay.LocalToast
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.result.Result
import co.typie.result.onOk
import co.typie.result.result
import co.typie.result.withDefaultExceptionHandler
import com.apollographql.apollo.ApolloClient
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import kotlin.time.Clock
import kotlin.time.Instant
import org.koin.compose.viewmodel.koinViewModel
import org.koin.core.annotation.KoinViewModel

@Composable
fun MarketingConsentGate() {
  val model = koinViewModel<MarketingConsentGateViewModel>()
  val toast = LocalToast.current
  val data = (model.query.state as? QueryState.Success)?.data ?: return
  var handledInSession by remember { mutableStateOf(false) }
  var pendingConsent by remember { mutableStateOf<Boolean?>(null) }

  val shouldShow = !handledInSession && shouldShowMarketingConsentPrompt(
    marketingConsentAskedAt = data.me.marketingConsentAskedAt,
    totalCharacterCount = data.me.usage.totalCharacterCount,
  )

  if (!shouldShow) return

  MarketingConsentModal(
    pendingConsent = pendingConsent,
    onConsent = { consent ->
      pendingConsent = consent
      model.updateMarketingConsent(consent)
        .withDefaultExceptionHandler(toast)
        .onOk {
          toast.show(ToastType.Success, marketingConsentToastMessage(consent))
          handledInSession = true
        }
      pendingConsent = null
    },
  )
}

internal fun marketingConsentToastMessage(
  marketingConsent: Boolean,
  now: Instant = Clock.System.now(),
): String {
  val action = if (marketingConsent) "동의" else "거부"
  return "${now.formatKoreanDate()}에 마케팅 수신 ${action}처리됐어요."
}

@KoinViewModel
class MarketingConsentGateViewModel(
  private val apolloClient: ApolloClient,
) : ViewModel() {
  val query = apolloClient.watchQuery(scope = viewModelScope) { MarketingConsentGate_Query() }

  suspend fun updateMarketingConsent(marketingConsent: Boolean): Result<Unit, Nothing> {
    return result {
      apolloClient.executeMutation(
        MarketingConsentGate_UpdateMarketingConsent_Mutation(
          input = UpdateMarketingConsentInput(marketingConsent = marketingConsent),
        ),
      )
    }
  }
}

@Composable
private fun MarketingConsentModal(
  pendingConsent: Boolean?,
  onConsent: suspend (Boolean) -> Unit,
) {
  val isSubmitting = pendingConsent != null

  Dialog(
    onDismissRequest = {},
    properties = DialogProperties(
      dismissOnBackPress = false,
      dismissOnClickOutside = false,
    ),
  ) {
    Column(
      modifier = Modifier
        .width(320.dp)
        .clip(RoundedCornerShape(24.dp))
        .background(AppTheme.colors.surfaceRaised)
        .border(1.dp, AppTheme.colors.borderSubtle, RoundedCornerShape(24.dp))
        .padding(24.dp),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      MarketingConsentIconCluster()

      Spacer(Modifier.height(18.dp))

      Text(
        text = "타이피 소식 받아보기",
        style = AppTheme.typography.title,
      )

      Spacer(Modifier.height(8.dp))

      Text(
        text = "새 기능, 글쓰기 팁, 할인 혜택 등\n다양한 소식을 전해드려요.",
        style = AppTheme.typography.body.copy(textAlign = TextAlign.Center),
        color = AppTheme.colors.textTertiary,
      )

      Spacer(Modifier.height(24.dp))

      Button(
        text = "받을게요",
        onClick = { onConsent(true) },
        loading = pendingConsent == true,
        enabled = !isSubmitting,
      )

      Spacer(Modifier.height(8.dp))

      Button(
        text = "안 받을게요",
        onClick = { onConsent(false) },
        variant = ButtonVariant.Secondary,
        loading = pendingConsent == false,
        enabled = !isSubmitting,
      )

      Spacer(Modifier.height(16.dp))

      Text(
        text = "나중에 설정에서 변경할 수 있어요",
        style = AppTheme.typography.caption.copy(textAlign = TextAlign.Center),
        color = AppTheme.colors.textTertiary,
      )
    }
  }
}

@Composable
private fun MarketingConsentIconCluster() {
  val icons = listOf(Lucide.Mail, Lucide.Bell, Lucide.Sparkles, Lucide.Zap, Lucide.Gift)

  Box(
    modifier = Modifier
      .width(120.dp)
      .height(32.dp),
    contentAlignment = Alignment.CenterStart,
  ) {
    icons.forEachIndexed { index, icon ->
      Box(
        modifier = Modifier
          .offset(x = (index * 22).dp)
          .size(32.dp)
          .clip(CircleShape)
          .background(AppTheme.colors.surfaceTinted)
          .border(2.dp, AppTheme.colors.surfaceRaised, CircleShape),
        contentAlignment = Alignment.Center,
      ) {
        Icon(
          icon = icon,
          modifier = Modifier.size(16.dp),
          tint = AppTheme.colors.textPrimary,
        )
      }
    }
  }
}
