package co.typie.screen.subscription.current_plan

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
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
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.QueryState
import co.typie.navigation.Nav
import co.typie.route.Route
import co.typie.screen.subscription.CurrentPlanFooter
import co.typie.screen.subscription.currentPlanDetailLines
import co.typie.screen.subscription.currentPlanFooter
import co.typie.service.CurrentSubscriptionStore
import co.typie.service.SubscriptionAvailability
import co.typie.service.SubscriptionState
import co.typie.service.shouldAutoCloseCurrentPlan
import co.typie.ui.component.Button
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.error
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
fun CurrentPlanScreen() {
  val nav = Nav.current
  val scrollState = rememberScrollState()
  val currentSubscriptionState = CurrentSubscriptionStore.state

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("이용권 정보", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  val dialog = LocalDialog.current

  LaunchedEffect(currentSubscriptionState) {
    if (currentSubscriptionState is QueryState.Error) {
      dialog.error(nav = nav, onRetry = { CurrentSubscriptionStore.refresh() })
    }
  }

  LaunchedEffect(currentSubscriptionState) {
    if (shouldAutoCloseCurrentPlan(currentSubscriptionState)) {
      nav.pop()
    }
  }

  Screen(
    scrollState = scrollState,
    loading = currentSubscriptionState !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    val subscription = (currentSubscriptionState as? QueryState.Success)?.data ?: return@Screen
    val availability = subscription.availability
    val footer = availability?.let(::currentPlanFooter)
    val detailLines =
      currentPlanDetailLines(
        availability = availability ?: SubscriptionAvailability.Manual,
        fee = subscription.fee ?: 0,
        state = subscription.state ?: SubscriptionState.Active,
        expiresAt = subscription.expiresAt ?: return@Screen,
      )

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
              subscription.planName ?: "",
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

        if (footer != null) {
          CardDivider()
          CurrentPlanFooterSection(
            footer = footer,
            onCancelClick = { nav.navigate(Route.CancelPlan) },
            onChangeClick = { nav.navigate(Route.EnrollPlan) },
            onUpgradeClick = { nav.navigate(Route.EnrollPlan) },
          )
        }
      }
    }

    Spacer(Modifier.height(72.dp))
  }
}

@Composable
private fun CurrentPlanFooterSection(
  footer: CurrentPlanFooter,
  onCancelClick: suspend () -> Unit,
  onChangeClick: suspend () -> Unit,
  onUpgradeClick: suspend () -> Unit,
) {
  when (footer) {
    is CurrentPlanFooter.Actions -> {
      Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        CurrentPlanFooterAction(
          label = footer.labels.first(),
          modifier = Modifier.weight(1f),
          onClick = onCancelClick,
        )

        Box(
          modifier =
            Modifier.size(width = 1.dp, height = 20.dp).background(AppTheme.colors.borderSubtle)
        )

        CurrentPlanFooterAction(
          label = footer.labels.last(),
          modifier = Modifier.weight(1f),
          onClick = onChangeClick,
        )
      }
    }

    is CurrentPlanFooter.Note -> {
      Text(
        text = footer.text,
        style = AppTheme.typography.body,
        color = AppTheme.colors.textTertiary,
        modifier = Modifier.padding(16.dp),
      )
    }

    is CurrentPlanFooter.Upgrade -> {
      Button(text = footer.label, onClick = onUpgradeClick, modifier = Modifier.padding(16.dp))
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
