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
import co.typie.datetime.toLocalDate
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.type.PlanAvailability
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.time.Clock
import kotlinx.datetime.daysUntil

@Composable
fun TrialRemainingChip() {
  val dialog = LocalDialog.current

  val entitlement = SubscriptionService.entitlement
  if (entitlement !is Entitlement.Active) return
  if (entitlement.subscription.availability != PlanAvailability.TRIAL) return

  val subscription = entitlement.subscription
  val now = Clock.System.now()
  if (!(subscription.expiresAt - now).isPositive()) return
  val daysLeft = now.toLocalDate().daysUntil(subscription.expiresAt.toLocalDate())

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
          text = if (daysLeft <= 0) "· 오늘 종료" else "· ${daysLeft}일 남음",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textOnInverse.copy(alpha = .65f),
        )
      }
    }
  }
}
