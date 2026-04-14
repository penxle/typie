package co.typie.domain.subscription

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.graphql.Apollo
import co.typie.graphql.EnrollPlanScreen_SubscribePlanWithTrial_Mutation
import co.typie.graphql.PlaceholderResolver
import co.typie.graphql.PlanUpgradeSheet_Query
import co.typie.graphql.TypieError
import co.typie.graphql.builder.Data
import co.typie.graphql.builder.buildUser
import co.typie.graphql.executeMutation
import co.typie.graphql.watchQuery
import co.typie.icons.Lucide
import co.typie.result.DEFAULT_ERROR_MESSAGE
import co.typie.result.Result
import co.typie.result.loading
import co.typie.result.onErr
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CardDivider
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

private const val TRIAL_START_CONFIRM_TITLE = "무료 체험을 시작하시겠어요?"
private const val TRIAL_START_CONFIRM_MESSAGE =
  "결제 수단 등록 없이 2주간 타이피의 모든 기능을 무료로 이용할 수 있어요. 체험 종료 후 자동 결제되지 않아요."
private const val TRIAL_START_CONFIRM_ACTION = "시작하기"
private const val DEFAULT_PLAN_UPGRADE_TITLE = "플랜 업그레이드가 필요해요"

sealed interface PlanUpgradeSheetResult {
  data object Upgrade : PlanUpgradeSheetResult

  data object TrialStarted : PlanUpgradeSheetResult
}

sealed interface PlanUpgradeTrialError {
  data object ServerError : PlanUpgradeTrialError
}

class PlanUpgradeSheetViewModel : ViewModel() {
  val query =
    Apollo.watchQuery(scope = viewModelScope, placeholderData = placeholderData()) {
      PlanUpgradeSheet_Query()
    }

  var isStartingTrial by mutableStateOf(false)
    private set

  var trialCompleted by mutableStateOf(false)
    private set

  suspend fun startTrial(): Result<Unit, PlanUpgradeTrialError> {
    return loading({ isStartingTrial = it }) {
      try {
        Apollo.executeMutation(EnrollPlanScreen_SubscribePlanWithTrial_Mutation())
        SubscriptionService.refresh()
        query.refetch()
        trialCompleted = true
      } catch (e: TypieError) {
        raise(PlanUpgradeTrialError.ServerError)
      }
    }
  }
}

private fun placeholderData() =
  PlanUpgradeSheet_Query.Data(PlaceholderResolver) { me = buildUser { canStartTrial = false } }

@Composable
context(_: SheetScope<PlanUpgradeSheetResult>)
fun PlanUpgradeSheet(title: String = DEFAULT_PLAN_UPGRADE_TITLE, message: String) {
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val model = viewModel { PlanUpgradeSheetViewModel() }
  val scope = rememberCoroutineScope()
  val canStartTrial = model.query.data.me.canStartTrial

  LaunchedEffect(model.trialCompleted) {
    if (model.trialCompleted) {
      complete(PlanUpgradeSheetResult.TrialStarted)
    }
  }

  SheetLayout(
    header = {
      Column {
        SubscriptionBadgeRow()
        SheetBar(
          center = {
            Text(
              text = title,
              style = AppTheme.typography.title,
              color = AppTheme.colors.textPrimary,
              overflow = TextOverflow.Ellipsis,
              maxLines = 1,
            )
          }
        )
        Text(
          text = message,
          style = AppTheme.typography.caption.copy(textAlign = TextAlign.Center),
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.fillMaxWidth(),
        )
      }
    },
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
    Column(
      modifier =
        Modifier.fillMaxWidth()
          .border(1.dp, AppTheme.colors.borderStrong, AppShapes.rounded(AppShapes.md))
          .padding(16.dp),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Text(text = "타이피 FULL ACCESS", style = AppTheme.typography.title)

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderStrong)

      SubscriptionFeatureList(features = fullPlanFeatures, iconSize = 16.dp, rowSpacing = 8.dp)
    }
  }
}
