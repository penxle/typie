package co.typie.screen.subscription.cancelplan

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.formatKoreanDate
import co.typie.domain.subscription.Entitlement
import co.typie.domain.subscription.SubscriptionFeatureList
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.fullPlanFeatures
import co.typie.ext.verticalScroll
import co.typie.navigation.Nav
import co.typie.platform.Platform
import co.typie.platform.PlatformModule
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
fun CancelPlanScreen() {
  val model = viewModel { CancelPlanViewModel() }

  val scrollState = rememberScrollState()

  val nav = Nav.current
  val uriHandler = LocalUriHandler.current

  LaunchedEffect(SubscriptionService.entitlement) {
    if (SubscriptionService.entitlement is Entitlement.Expired) {
      nav.pop()
    }
  }

  ProvideTopBar(
    center = { Text("이용권 해지", style = AppTheme.typography.title) },
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
      val subscription = SubscriptionService.subscription ?: return@Screen

      Text("이용권 해지", style = AppTheme.typography.display)

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(20.dp),
          verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
          Text("정말 해지하시겠어요?", style = AppTheme.typography.heading)
          Text(
            "해지 시 다음 혜택을 더 이상 받을 수 없어요.",
            style = AppTheme.typography.body,
            color = AppTheme.colors.textMuted,
          )
        }
      }

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(20.dp),
          verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          Text("이용 중인 혜택", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

          Column(
            modifier = Modifier.fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(8.dp),
          ) {
            SubscriptionFeatureList(features = fullPlanFeatures)
          }
        }
      }

      Text(
        text =
          "지금 해지하더라도 ${subscription.expiresAt.formatKoreanDate()}까지는 계속해서 ${subscription.planName} 혜택을 이용할 수 있어요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textMuted,
      )

      Button(
        text = "스토어로 이동해서 해지하기",
        variant = ButtonVariant.Danger,
        onClick = {
          when (PlatformModule.platform) {
            Platform.Android ->
              uriHandler.openUri(
                "https://play.google.com/store/account/subscriptions?package=co.typie&sku=plan.full"
              )
            Platform.iOS -> uriHandler.openUri("https://apps.apple.com/account/subscriptions")
            Platform.Desktop -> {}
          }
        },
      )

      Button(text = "계속 이용하기", variant = ButtonVariant.Secondary, onClick = { nav.pop() })
    }
  }
}
