package co.typie.screen.subscription.currentplan

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.formatKoreanDate
import co.typie.datetime.toLocalDate
import co.typie.domain.subscription.Subscription
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.grantsAccess
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.comma
import co.typie.ext.pressScale
import co.typie.ext.verticalScroll
import co.typie.graphql.type.PlanAvailability
import co.typie.graphql.type.SubscriptionState
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.time.Clock
import kotlin.time.Instant

// LIFETIME·MANUAL 은 주기 종료를 sentinel(9999-12-31)로 저장한다. 값은 유지하되 날짜로 렌더하지 않는다.
private const val INDEFINITE_PERIOD_YEAR = 9999

internal fun isIndefinitePeriod(currentPeriodEndsAt: Instant): Boolean =
  currentPeriodEndsAt.toLocalDate().year >= INDEFINITE_PERIOD_YEAR

internal fun subscriptionPeriodLine(subscription: Subscription, now: Instant): String =
  when {
    isIndefinitePeriod(subscription.currentPeriodEndsAt) -> "무기한"
    // 갱신 창이 열리기 전의 ACTIVE 는 기간이 지난 채로 권한을 유지한다 — 지난 날짜를 다음 결제일로 쓰지 않는다.
    subscription.state == SubscriptionState.ACTIVE ->
      if (subscription.currentPeriodEndsAt <= now) "갱신 처리 중"
      else "다음 결제일: ${subscription.currentPeriodEndsAt.formatKoreanDate()}"
    subscription.state == SubscriptionState.IN_GRACE_PERIOD -> "결제를 다시 시도하고 있어요"
    subscription.state == SubscriptionState.WILL_ACTIVATE -> "플랜 전환 처리 중"
    else -> "해지 예정일: ${subscription.currentPeriodEndsAt.formatKoreanDate()}"
  }

@Composable
fun CurrentPlanScreen() {
  val model = viewModel { CurrentPlanViewModel() }
  val scrollState = rememberScrollState()
  val nav = Nav.current

  ProvideTopBar(
    center = { Text("이용권 정보", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  LaunchedEffect(SubscriptionService.entitlement) {
    if (!SubscriptionService.entitlement.grantsAccess()) {
      nav.pop()
    }
  }

  Screen(loadable = model.query) { innerPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(innerPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      val subscription = SubscriptionService.subscription ?: return@Screen

      Text("이용권 정보", style = AppTheme.typography.display)

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.fillMaxWidth()) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            Column(
              modifier = Modifier.fillMaxWidth(),
              verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
              Text("현재 이용권", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

              Text(
                subscription.planName,
                style = AppTheme.typography.heading,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }

            Column(
              modifier = Modifier.fillMaxWidth(),
              verticalArrangement = Arrangement.spacedBy(2.dp),
            ) {
              val detailLines =
                if (subscription.availability == PlanAvailability.TRIAL) {
                  listOf("무료 체험이 ${subscription.currentPeriodEndsAt.formatKoreanDate()}에 종료돼요.")
                } else {
                  listOf(
                    "이용권 가격: ${subscription.fee.comma}원",
                    subscriptionPeriodLine(subscription, Clock.System.now()),
                  )
                }

              detailLines.forEach { line ->
                Text(
                  text = line,
                  style = AppTheme.typography.body,
                  color = AppTheme.colors.textMuted,
                )
              }
            }
          }

          CardDivider()

          Footer(subscription = subscription)
        }
      }
    }
  }
}

@Composable
private fun Footer(subscription: Subscription) {
  val nav = Nav.current

  when (subscription.availability) {
    PlanAvailability.IN_APP_PURCHASE -> {
      Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        FooterButton(
          label = "해지하기",
          modifier = Modifier.weight(1f),
          onClick = { nav.navigate(Route.CancelPlan) },
        )

        Box(
          modifier =
            Modifier.size(width = 1.dp, height = 20.dp).background(AppTheme.colors.borderHairline)
        )

        FooterButton(
          label = "변경하기",
          modifier = Modifier.weight(1f),
          onClick = { nav.navigate(Route.EnrollPlan) },
        )
      }
    }

    PlanAvailability.BILLING_KEY -> {
      Text(
        text = "웹사이트에서 가입한 이용권이에요.\n정보 변경이 필요할 경우 웹사이트에서 진행해주세요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textMuted,
        modifier = Modifier.padding(16.dp),
      )
    }

    PlanAvailability.MANUAL -> {
      Text(
        text = "정보 변경을 할 수 없는 이용권이에요.\n정보 변경이 필요할 경우 고객센터에 문의해주세요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textMuted,
        modifier = Modifier.padding(16.dp),
      )
    }

    PlanAvailability.TRIAL -> {
      Button(
        text = "지금 업그레이드",
        onClick = { nav.navigate(Route.EnrollPlan) },
        modifier = Modifier.padding(16.dp),
      )
    }

    else -> {
      Text(
        text = "정보 변경을 할 수 없는 이용권이에요.\n정보 변경이 필요할 경우 고객센터에 문의해주세요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textMuted,
        modifier = Modifier.padding(16.dp),
      )
    }
  }
}

@Composable
private fun FooterButton(
  label: String,
  modifier: Modifier = Modifier,
  onClick: suspend () -> Unit,
) {
  InteractionScope {
    Box(
      modifier = modifier.clickable(onClick).padding(vertical = 12.dp).pressScale(),
      contentAlignment = Alignment.Center,
    ) {
      Text(text = label, style = AppTheme.typography.action, color = AppTheme.colors.textMuted)
    }
  }
}
