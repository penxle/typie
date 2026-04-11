package co.typie.screen.subscription

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
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
import co.typie.ui.component.ConfirmModal
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.ActionHeader
import co.typie.ui.component.sheet.SheetHostState
import co.typie.ui.component.sheet.SheetInsetPolicy
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPresentation
import co.typie.ui.component.sheet.completedOrNull
import co.typie.ui.component.sheet.sheetPresentation
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CancellationException
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

private fun planUpgradeSheet(
  title: String = DEFAULT_PLAN_UPGRADE_TITLE,
  message: String,
): SheetPresentation<PlanUpgradeSheetResult> = sheetPresentation {
  val toast = LocalToast.current
  val model = viewModel { PlanUpgradeSheetViewModel() }
  val scope = rememberCoroutineScope()
  var showTrialStartConfirm by remember { mutableStateOf(false) }
  val canStartTrial = SubscriptionService.canStartTrial(model.query.data.me.canStartTrial)
  val dismissResult = planUpgradeDismissResult(model.celebration)

  LaunchedEffect(dismissResult) {
    val result = dismissResult ?: return@LaunchedEffect
    complete(result)
  }

  SheetLayout(
    fillHeight = true,
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
            onClick = { showTrialStartConfirm = true },
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

  if (showTrialStartConfirm) {
    ConfirmModal(
      title = TRIAL_START_CONFIRM_TITLE,
      message = TRIAL_START_CONFIRM_MESSAGE,
      confirmText = TRIAL_START_CONFIRM_ACTION,
      onConfirm = {
        showTrialStartConfirm = false
        scope.launch {
          model.startTrial().withDefaultExceptionHandler(toast).onErr { error ->
            when (error) {
              PlanUpgradeTrialError.ServerError ->
                toast.show(ToastType.Error, DEFAULT_ERROR_MESSAGE)
            }
          }
        }
      },
      onDismiss = { showTrialStartConfirm = false },
    )
  }
}

suspend fun SheetHostState.showPlanUpgradeSheet(
  title: String = DEFAULT_PLAN_UPGRADE_TITLE,
  message: String,
): PlanUpgradeSheetResult? {
  val result =
    present(planUpgradeSheet(title = title, message = message)).completedOrNull() ?: return null

  if (result is PlanUpgradeSheetResult.TrialStarted) {
    try {
      present(
        subscriptionCelebrationSheet(
          title = result.celebration.title,
          message = result.celebration.message,
        )
      )
    } catch (_: CancellationException) {
      // Celebration sheet dismissal does not affect the original result.
    }
  }

  return result
}
