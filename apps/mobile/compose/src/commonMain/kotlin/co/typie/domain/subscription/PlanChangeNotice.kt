package co.typie.domain.subscription

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.graphql.type.PlanAvailability
import co.typie.storage.Preference
import co.typie.ui.component.Button
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.DialogScope
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.dismiss
import co.typie.ui.component.dialog.resolve
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.time.Instant
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first

internal val LEGACY_TRIAL_CUTOFF = Instant.parse("2026-07-13T00:00:00+09:00")

internal fun Subscription.isLegacyTrial(): Boolean =
  availability == PlanAvailability.TRIAL && startsAt <= LEGACY_TRIAL_CUTOFF

suspend fun Dialog.presentPlanChangeNotice(showSubscribe: Boolean): Boolean {
  val result =
    present(dismissible = true) { PlanChangeNoticeContent(showSubscribe = showSubscribe) }
  return result is DialogResult.Resolved
}

@Composable
fun PlanChangeNoticeHost() {
  val dialog = LocalDialog.current

  LaunchedEffect(Unit) {
    if (Preference.planChangeNoticeShown) return@LaunchedEffect

    val initial = SubscriptionService.entitlement
    if (initial is Entitlement.Active && !initial.subscription.isLegacyTrial()) {
      return@LaunchedEffect
    }

    val subscription = snapshotFlow {
      (SubscriptionService.entitlement as? Entitlement.Active)?.subscription
    }.filterNotNull().first()
    if (!subscription.isLegacyTrial()) return@LaunchedEffect

    Preference.planChangeNoticeShown = true
    dialog.presentPlanChangeNotice(showSubscribe = false)
  }
}

@Composable
context(_: DialogScope<Unit>)
private fun PlanChangeNoticeContent(showSubscribe: Boolean) {
  Column(
    modifier =
      Modifier.fillMaxWidth()
        .clip(AppShapes.rounded(AppShapes.lg))
        .background(AppTheme.colors.surfaceDefault)
        .padding(horizontal = 20.dp, vertical = 28.dp),
    horizontalAlignment = Alignment.CenterHorizontally,
  ) {
    Text(text = "구독 플랜 개편 안내", style = AppTheme.typography.title)

    Spacer(Modifier.height(12.dp))

    Text(
      text = "무료 플랜은 종료되고,\n월 구독료는 2,900원으로 낮아졌어요.\n\n기존 이용자는 7월 27일까지\n모든 기능을 무료로 이용할 수 있어요.",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textMuted,
      textAlign = TextAlign.Center,
    )

    Spacer(Modifier.height(24.dp))

    if (showSubscribe) {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        OutlineButton(text = "좀 더 둘러볼게요", onClick = { dismiss() })

        Button(text = "구독하고 계속 사용하기", onClick = { resolve(Unit) })
      }
    } else {
      Button(text = "확인했어요", onClick = { dismiss() })
    }
  }
}

@Composable
private fun OutlineButton(text: String, onClick: () -> Unit) {
  InteractionScope {
    Box(
      modifier =
        Modifier.fillMaxWidth()
          .height(48.dp)
          .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.lg))
          .border(1.dp, AppTheme.colors.borderDefault, AppShapes.rounded(AppShapes.lg))
          .clickable { onClick() },
      contentAlignment = Alignment.Center,
    ) {
      Text(
        text = text,
        style = AppTheme.typography.action,
        color = AppTheme.colors.textDefault,
        modifier = Modifier.pressScale(0.95f),
      )
    }
  }
}
