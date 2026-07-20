package co.typie.domain.subscription

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.layout.layout
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import co.typie.datetime.toLocalDate
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.type.PlanAvailability
import co.typie.storage.Preference
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.time.Clock
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.datetime.LocalDate
import kotlinx.datetime.daysUntil

internal fun shouldShowTrialReminder(
  daysLeft: Int,
  today: LocalDate,
  lastShownDate: String?,
): Boolean = daysLeft <= 3 && lastShownDate != today.toString()

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

  val openSubscribe: suspend () -> Unit = {
    if (subscription.isLegacyTrial()) {
      if (dialog.presentPlanChangeNotice(showSubscribe = true)) {
        SubscriptionService.requestSubscribeSheet()
      }
    } else {
      SubscriptionService.requestSubscribeSheet()
    }
  }

  val scope = rememberCoroutineScope()
  var reminderVisible by remember { mutableStateOf(false) }

  LaunchedEffect(daysLeft) {
    val today = Clock.System.now().toLocalDate()
    if (!shouldShowTrialReminder(daysLeft, today, Preference.trialReminderLastShownDate)) {
      return@LaunchedEffect
    }
    delay(500)
    Preference.trialReminderLastShownDate = today.toString()
    reminderVisible = true
    delay(5_000)
    reminderVisible = false
  }

  Box {
    InteractionScope {
      Row(
        modifier =
          Modifier.background(AppTheme.colors.surfaceInverse, AppShapes.circle)
            .clickable(onClick = openSubscribe)
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

    Box(
      modifier =
        Modifier.align(Alignment.BottomEnd).layout { measurable, _ ->
          val placeable = measurable.measure(Constraints())
          layout(0, 0) { placeable.placeRelative(x = -placeable.width, y = 8.dp.roundToPx()) }
        }
    ) {
      AnimatedVisibility(
        visible = reminderVisible,
        enter = fadeIn(tween(200)) + slideInVertically(animationSpec = spring()) { -it / 3 },
        exit = fadeOut(tween(200)) + slideOutVertically(animationSpec = tween(200)) { -it / 3 },
      ) {
        TrialReminderBalloon(
          daysLeft = daysLeft,
          legacy = subscription.isLegacyTrial(),
          onTap = {
            reminderVisible = false
            scope.launch { openSubscribe() }
          },
        )
      }
    }
  }
}

@Composable
private fun TrialReminderBalloon(daysLeft: Int, legacy: Boolean, onTap: () -> Unit) {
  val shape = AppShapes.rounded(AppShapes.lg)

  InteractionScope {
    Box {
      Row(
        modifier =
          Modifier.padding(top = 4.dp)
            .shadow(AppTheme.shadows.lg, shape)
            .background(AppTheme.colors.surfaceInverse, shape)
            .clickable(onClick = { onTap() })
            .padding(horizontal = 14.dp, vertical = 10.dp)
            .pressScale()
      ) {
        Text(
          buildAnnotatedString {
            append(if (legacy) "무료 이용 기간이 " else "무료 체험이 ")
            withStyle(SpanStyle(fontWeight = FontWeight.Bold)) {
              append(if (daysLeft <= 0) "오늘" else "${daysLeft}일")
            }
            append(if (daysLeft <= 0) " 끝나요" else " 남았어요")
          },
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textOnInverse,
          maxLines = 1,
        )
      }

      Box(
        modifier =
          Modifier.align(Alignment.TopEnd)
            .padding(end = 20.dp)
            .size(9.dp)
            .rotate(45f)
            .background(AppTheme.colors.surfaceInverse)
      )
    }
  }
}
