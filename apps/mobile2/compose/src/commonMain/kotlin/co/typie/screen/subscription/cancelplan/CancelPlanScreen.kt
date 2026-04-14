package co.typie.screen.subscription.cancelplan

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.formatKoreanDate
import co.typie.domain.subscription.SubscriptionFeatureList
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.SubscriptionServiceState
import co.typie.domain.subscription.fullPlanFeatures
import co.typie.ext.verticalScroll
import co.typie.navigation.Nav
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun CancelPlanScreen() {
  val nav = Nav.current
  val toast = LocalToast.current
  val model = viewModel { CancelPlanViewModel() }
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()
  val lifecycleOwner = LocalLifecycleOwner.current
  val currentSubscriptionState = SubscriptionService.state

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("이용권 해지", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  DisposableEffect(lifecycleOwner) {
    val observer = LifecycleEventObserver { _, event ->
      if (event == Lifecycle.Event.ON_RESUME) {
        model.onResumed()
      }
    }
    lifecycleOwner.lifecycle.addObserver(observer)
    onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
  }

  LaunchedEffect(model.shouldClose) {
    if (model.shouldClose) {
      model.consumeCloseRequest()
      nav.pop()
    }
  }

  LaunchedEffect(model.errorMessage) {
    val errorMessage = model.errorMessage ?: return@LaunchedEffect
    toast.show(ToastType.Error, errorMessage)
    model.consumeErrorMessage()
  }

  Screen(query = model.query) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      val subscription =
        (currentSubscriptionState as? SubscriptionServiceState.Subscribed)?.subscription
          ?: return@Screen
      Text("이용권 해지", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(18.dp),
          verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
          Text("정말 해지하시겠어요?", style = AppTheme.typography.heading)
          Text(
            "해지 시 다음 혜택을 더 이상 받을 수 없어요.",
            style = AppTheme.typography.body,
            color = AppTheme.colors.textSecondary,
          )
        }
      }

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(
          modifier = Modifier.fillMaxWidth().padding(18.dp),
          verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
          Text(
            "이용 중인 혜택",
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textTertiary,
          )

          Column(
            modifier = Modifier.fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(10.dp),
          ) {
            SubscriptionFeatureList(features = fullPlanFeatures)
          }
        }
      }

      Text(
        text =
          "지금 해지하더라도 ${subscription.expiresAt.formatKoreanDate()}까지는 계속해서 ${subscription.planName} 혜택을 이용할 수 있어요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textTertiary,
      )

      Button(
        text = "스토어로 이동해서 해지하기",
        variant = ButtonVariant.Danger,
        loading = model.isOpeningSubscriptionManagement,
        loadingText = "스토어로 이동 중...",
        onClick = {
          // TODO: Mixpanel/Appsflyer cancel_plan_try
          scope.launch { model.openSubscriptionManagement().withDefaultExceptionHandler(toast) }
        },
      )

      Button(text = "계속 이용하기", variant = ButtonVariant.Secondary, onClick = { nav.pop() })

      Spacer(Modifier.height(72.dp))
    }
  }
}
