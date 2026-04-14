package co.typie.screen.subscription.currentplan

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
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
import co.typie.domain.subscription.SubscriptionService
import co.typie.domain.subscription.SubscriptionServiceState
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
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
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
fun CurrentPlanScreen() {
  val model = viewModel { CurrentPlanViewModel() }
  val nav = Nav.current
  val scrollState = rememberScrollState()
  val currentSubscriptionState = SubscriptionService.state

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("이용권 정보", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  LaunchedEffect(currentSubscriptionState) {
    if (currentSubscriptionState is SubscriptionServiceState.NotSubscribed) {
      nav.pop()
    }
  }

  Screen(query = model.query) { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      val subscription =
        (currentSubscriptionState as? SubscriptionServiceState.Subscribed)?.subscription
          ?: return@Screen

      val detailLines =
        if (subscription.availability == PlanAvailability.TRIAL) {
          listOf("무료 체험이 ${subscription.expiresAt.formatKoreanDate()}에 종료돼요.")
        } else {
          listOf(
            "이용권 가격: ${subscription.fee.formatGrouped()}원",
            if (subscription.state == SubscriptionState.ACTIVE) {
              "다음 결제일: ${subscription.expiresAt.formatKoreanDate()}"
            } else {
              "해지 예정일: ${subscription.expiresAt.formatKoreanDate()}"
            },
          )
        }

      Text("이용권 정보", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

      CardSurface(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.fillMaxWidth()) {
          Column(
            modifier = Modifier.fillMaxWidth().padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            Column(
              modifier = Modifier.fillMaxWidth(),
              verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
              Text(
                "현재 이용권",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )

              Text(
                subscription.planName,
                style = AppTheme.typography.heading,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
              )
            }

            Column(
              modifier = Modifier.fillMaxWidth(),
              verticalArrangement = Arrangement.spacedBy(3.dp),
            ) {
              detailLines.forEach { line ->
                Text(
                  text = line,
                  style = AppTheme.typography.body,
                  color = AppTheme.colors.textTertiary,
                )
              }
            }
          }

          CardDivider()
          CurrentPlanFooterSection(
            availability = subscription.availability,
            onCancelClick = { nav.navigate(Route.CancelPlan) },
            onChangeClick = { nav.navigate(Route.EnrollPlan) },
            onUpgradeClick = { nav.navigate(Route.EnrollPlan) },
          )
        }
      }

      Spacer(Modifier.height(72.dp))
    }
  }
}

@Composable
private fun CurrentPlanFooterSection(
  availability: PlanAvailability,
  onCancelClick: suspend () -> Unit,
  onChangeClick: suspend () -> Unit,
  onUpgradeClick: suspend () -> Unit,
) {
  when (availability) {
    PlanAvailability.IN_APP_PURCHASE -> {
      Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        CurrentPlanFooterAction(
          label = "해지하기",
          modifier = Modifier.weight(1f),
          onClick = onCancelClick,
        )

        Box(
          modifier =
            Modifier.size(width = 1.dp, height = 20.dp).background(AppTheme.colors.borderSubtle)
        )

        CurrentPlanFooterAction(
          label = "변경하기",
          modifier = Modifier.weight(1f),
          onClick = onChangeClick,
        )
      }
    }

    PlanAvailability.BILLING_KEY -> {
      Text(
        text = "웹사이트에서 가입한 이용권이에요.\n정보 변경이 필요할 경우 웹사이트에서 진행해주세요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textTertiary,
        modifier = Modifier.padding(16.dp),
      )
    }

    PlanAvailability.MANUAL -> {
      Text(
        text = "정보 변경을 할 수 없는 이용권이에요.\n정보 변경이 필요할 경우 고객센터에 문의해주세요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textTertiary,
        modifier = Modifier.padding(16.dp),
      )
    }

    PlanAvailability.TRIAL -> {
      Button(text = "지금 업그레이드", onClick = onUpgradeClick, modifier = Modifier.padding(16.dp))
    }

    else -> {
      Text(
        text = "정보 변경을 할 수 없는 이용권이에요.\n정보 변경이 필요할 경우 고객센터에 문의해주세요.",
        style = AppTheme.typography.body,
        color = AppTheme.colors.textTertiary,
        modifier = Modifier.padding(16.dp),
      )
    }
  }
}

@Composable
private fun CurrentPlanFooterAction(
  label: String,
  modifier: Modifier = Modifier,
  onClick: suspend () -> Unit,
) {
  InteractionScope {
    Box(
      modifier = modifier.clickable(onClick).padding(vertical = 12.dp).pressScale(),
      contentAlignment = Alignment.Center,
    ) {
      Text(text = label, style = AppTheme.typography.action, color = AppTheme.colors.textSecondary)
    }
  }
}

private fun Int.formatGrouped(): String {
  val text = toString()
  val builder = StringBuilder()

  text.forEachIndexed { index, char ->
    if (index > 0 && (text.length - index) % 3 == 0) {
      builder.append(',')
    }
    builder.append(char)
  }

  return builder.toString()
}
