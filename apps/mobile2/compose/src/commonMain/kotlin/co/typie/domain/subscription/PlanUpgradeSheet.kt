package co.typie.domain.subscription

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
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
import co.typie.ui.component.Divider
import co.typie.ui.component.Text
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.sheet.SheetBarTextButton
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.component.sheet.dismiss
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastType
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.PaperlogyFontFamily

private const val TRIAL_START_CONFIRM_TITLE = "무료 체험을 시작하시겠어요?"
private const val TRIAL_START_CONFIRM_MESSAGE =
  "결제 수단 등록 없이 2주간 타이피의 모든 기능을 무료로 이용할 수 있어요. 체험 종료 후 자동 결제되지 않아요."
private const val TRIAL_START_CONFIRM_ACTION = "시작하기"

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
fun PlanUpgradeSheet(
  title: String,
  benefits: List<PlanUpgradeBenefit>,
  preview: (@Composable () -> Unit)? = null,
) {
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val model = viewModel { PlanUpgradeSheetViewModel() }
  val canStartTrial = model.query.data.me.canStartTrial

  LaunchedEffect(model.trialCompleted) {
    if (model.trialCompleted) complete(PlanUpgradeSheetResult.TrialStarted)
  }

  SheetLayout(
    padding = SheetPadding(header = PaddingValues(0.dp)),
    header = { InkHeroStrip(title = title) },
    footer = {
      Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Box {
          Button(
            text = if (canStartTrial) "2주 무료 체험 시작하기" else "FULL ACCESS 시작하기",
            textStyle = TextStyle(fontFamily = PaperlogyFontFamily),
            loading = model.isStartingTrial,
            loadingText = "무료 체험 시작 중...",
            height = 54.dp,
            onClick = {
              if (canStartTrial) {
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
              } else {
                complete(PlanUpgradeSheetResult.Upgrade)
              }
            },
          )

          if (canStartTrial) {
            TrialAvailablePill(modifier = Modifier.align(Alignment.TopCenter).offset(y = (-11).dp))
          }
        }

        SheetBarTextButton(
          text = "나중에 할게요",
          modifier = Modifier.fillMaxWidth(),
          color = AppTheme.colors.textMuted,
          onClick = { dismiss() },
        )
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp)) {
      preview?.invoke()

      Spacer(Modifier.height(12.dp))

      PlanUpgradeBenefitList(benefits = benefits)
    }
  }
}

@Composable
private fun InkHeroStrip(title: String) {
  Column(
    modifier =
      Modifier.fillMaxWidth()
        .background(AppTheme.colors.surfaceInverse)
        .padding(horizontal = 20.dp, vertical = 24.dp),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    FullAccessBadge()
    Text(
      text = title,
      style = AppTheme.typography.heading.copy(fontFamily = PaperlogyFontFamily),
      color = AppTheme.colors.textOnInverse,
    )
  }
}

@Composable
private fun FullAccessBadge() {
  Row(
    modifier =
      Modifier.border(
          1.dp,
          AppTheme.colors.textOnInverse.copy(alpha = 0.3f),
          RoundedCornerShape(3.dp),
        )
        .padding(horizontal = 7.dp, vertical = 3.dp),
    horizontalArrangement = Arrangement.spacedBy(5.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(icon = Lucide.Check, modifier = Modifier.size(8.dp), tint = AppTheme.colors.textOnInverse)
    Text(
      text = "FULL ACCESS",
      style = AppTheme.typography.micro.copy(fontWeight = FontWeight.Bold, letterSpacing = 0.6.sp),
      color = AppTheme.colors.textOnInverse,
    )
  }
}

@Composable
private fun TrialAvailablePill(modifier: Modifier = Modifier) {
  Row(
    modifier =
      modifier
        .background(AppTheme.colors.surfaceCanvas, RoundedCornerShape(999.dp))
        .border(1.dp, AppTheme.colors.borderDefault, RoundedCornerShape(999.dp))
        .padding(horizontal = 10.dp, vertical = 4.dp),
    horizontalArrangement = Arrangement.spacedBy(4.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Icon(icon = Lucide.Zap, modifier = Modifier.size(10.dp), tint = AppTheme.colors.textDefault)
    Text(
      text = "2주 무료 체험 가능",
      style = AppTheme.typography.micro.copy(fontWeight = FontWeight.Bold),
      color = AppTheme.colors.textDefault,
    )
  }
}

@Composable
private fun PlanUpgradeBenefitList(benefits: List<PlanUpgradeBenefit>) {
  Column(modifier = Modifier.fillMaxWidth()) {
    benefits.forEachIndexed { index, benefit ->
      Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.Top,
      ) {
        Box(
          modifier =
            Modifier.size(28.dp).background(AppTheme.colors.surfaceInset, RoundedCornerShape(8.dp)),
          contentAlignment = Alignment.Center,
        ) {
          Icon(
            icon = benefit.icon,
            modifier = Modifier.size(15.dp),
            tint = AppTheme.colors.textDefault,
          )
        }
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
          Text(
            text = benefit.title,
            style = AppTheme.typography.label.copy(fontFamily = PaperlogyFontFamily),
            color = AppTheme.colors.textDefault,
          )
          Text(
            text = benefit.description,
            style = AppTheme.typography.caption,
            color = AppTheme.colors.textMuted,
          )
        }
      }
      if (index < benefits.lastIndex) {
        Divider(color = AppTheme.colors.borderHairline)
      }
    }
  }
}
