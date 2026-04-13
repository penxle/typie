package co.typie.screen.subscription

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.icons.Lucide
import co.typie.overlay.LocalToast
import co.typie.overlay.ToastType
import co.typie.result.DEFAULT_ERROR_MESSAGE
import co.typie.result.onErr
import co.typie.result.withDefaultExceptionHandler
import co.typie.route.Route
import co.typie.service.SubscriptionCelebration
import co.typie.service.SubscriptionService
import co.typie.service.TRIAL_START_CONFIRM_ACTION
import co.typie.service.TRIAL_START_CONFIRM_MESSAGE
import co.typie.service.TRIAL_START_CONFIRM_TITLE
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardDivider
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.SheetInsetPolicy
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

const val DEFAULT_PLAN_UPGRADE_TITLE = "플랜 업그레이드가 필요해요"

sealed interface PlanUpgradeSheetResult {
  data object Upgrade : PlanUpgradeSheetResult

  data class TrialStarted(val celebration: SubscriptionCelebration) : PlanUpgradeSheetResult
}

fun planUpgradeDismissResult(celebration: SubscriptionCelebration?): PlanUpgradeSheetResult? {
  return celebration?.let(PlanUpgradeSheetResult::TrialStarted)
}

fun planUpgradeRoute(result: PlanUpgradeSheetResult?): Route? {
  return when (result) {
    PlanUpgradeSheetResult.Upgrade -> Route.EnrollPlan
    is PlanUpgradeSheetResult.TrialStarted,
    null -> null
  }
}

@Composable
context(_: SheetScope<PlanUpgradeSheetResult>)
fun PlanUpgradeContent(title: String = DEFAULT_PLAN_UPGRADE_TITLE, message: String) {
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val model = viewModel { PlanUpgradeSheetViewModel() }
  val scope = rememberCoroutineScope()
  val canStartTrial = SubscriptionService.canStartTrial(model.query.data.me.canStartTrial)
  val dismissResult = planUpgradeDismissResult(model.celebration)

  LaunchedEffect(dismissResult) {
    val result = dismissResult ?: return@LaunchedEffect
    complete(result)
  }

  SheetLayout(
    bodyInsetPolicy = SheetInsetPolicy.Container,
    header = { ActionHeader(title = title) },
    footer = {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        if (canStartTrial) {
          Button(
            text = "2주 무료 체험하기",
            leading = { color -> Icon(icon = Lucide.Zap, tint = color) },
            loading = model.isStartingTrial,
            loadingText = "무료 체험 시작 중...",
            onClick = {
              scope.launch {
                val result =
                  dialog.confirm(
                    title = TRIAL_START_CONFIRM_TITLE,
                    message = TRIAL_START_CONFIRM_MESSAGE,
                    confirmText = TRIAL_START_CONFIRM_ACTION,
                  )
                if (result is DialogResult.Resolved) {
                  model.startTrial().withDefaultExceptionHandler(toast).onErr { error ->
                    when (error) {
                      PlanUpgradeTrialError.ServerError ->
                        toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
                    }
                  }
                }
              }
            },
          )
        }

        Button(
          text = "업그레이드",
          variant = if (canStartTrial) ButtonVariant.Secondary else ButtonVariant.Primary,
          onClick = { complete(PlanUpgradeSheetResult.Upgrade) },
        )
      }
    },
  ) {
    SubscriptionBadgeRow()

    Text(
      text = message,
      style = AppTheme.typography.caption.copy(textAlign = TextAlign.Center),
      color = AppTheme.colors.textTertiary,
      modifier = Modifier.fillMaxWidth(),
    )

    Column(
      modifier =
        Modifier.fillMaxWidth()
          .border(1.dp, AppTheme.colors.borderStrong, RoundedCornerShape(8.dp))
          .padding(16.dp),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Text(text = "타이피 FULL ACCESS", style = AppTheme.typography.title)

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderStrong)

      SubscriptionFeatureList(features = fullPlanFeatures, iconSize = 16.dp, rowSpacing = 8.dp)
    }
  }
}
