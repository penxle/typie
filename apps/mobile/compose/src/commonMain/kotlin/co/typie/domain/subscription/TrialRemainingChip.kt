package co.typie.domain.subscription

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.type.PlanAvailability
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.ceil
import kotlin.time.Clock
import kotlin.time.DurationUnit

@Composable
fun TrialRemainingChip() {
  val dialog = LocalDialog.current

  val entitlement = SubscriptionService.entitlement
  if (entitlement !is Entitlement.Active) return
  if (entitlement.subscription.availability != PlanAvailability.TRIAL) return

  val subscription = entitlement.subscription
  val remaining = subscription.expiresAt - Clock.System.now()
  if (!remaining.isPositive()) return
  val days = ceil(remaining.toDouble(DurationUnit.DAYS)).toInt().coerceAtLeast(1)

  InteractionScope {
    Row(
      modifier =
        Modifier.background(AppTheme.colors.surfaceInverse, AppShapes.circle)
          .clickable {
            if (subscription.isLegacyTrial()) {
              if (dialog.presentPlanChangeNotice(showSubscribe = true)) {
                SubscriptionService.requestSubscribeSheet()
              }
            } else {
              SubscriptionService.requestSubscribeSheet()
            }
          }
          .padding(horizontal = 14.dp, vertical = 7.dp)
    ) {
      Row(
        modifier = Modifier.pressScale(),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Text(
          text = if (subscription.isLegacyTrial()) "무료 이용 기간" else "무료 체험 중",
          style = AppTheme.typography.caption.copy(fontWeight = FontWeight.SemiBold),
          color = AppTheme.colors.textOnInverse,
        )
        Text(
          text = "· ${days}일 남음",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textOnInverse.copy(alpha = .65f),
        )
      }
    }
  }
}
