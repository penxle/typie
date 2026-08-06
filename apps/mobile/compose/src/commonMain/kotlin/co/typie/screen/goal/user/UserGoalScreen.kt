package co.typie.screen.goal.user

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.kstToday
import co.typie.domain.goal.GoalColorState
import co.typie.domain.goal.GoalSection
import co.typie.domain.goal.USER_GOAL_HISTORY_DAYS
import co.typie.domain.goal.USER_GOAL_TREND_DAYS
import co.typie.domain.goal.UserGoalBarChart
import co.typie.domain.goal.UserGoalDay
import co.typie.domain.goal.UserGoalDots
import co.typie.domain.goal.UserGoalHistoryTable
import co.typie.domain.goal.dailyAdditionRows
import co.typie.domain.goal.dotDays
import co.typie.domain.goal.goalHeroNumberStyle
import co.typie.domain.goal.streaks
import co.typie.domain.goal.toUserGoalDays
import co.typie.domain.goal.todayProgress
import co.typie.ext.comma
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.icons.Lucide
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.CommaVisualTransformation
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.ProgressRing
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.confirm
import co.typie.ui.component.filterGoalDigits
import co.typie.ui.component.toGoalTargetOrNull
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.datetime.LocalDate

@Composable
fun UserGoalScreen() {
  val model = viewModel { UserGoalViewModel() }
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val scrollState = rememberScrollState()

  val loading = model.query.state !is QueryState.Success
  val me = model.query.data.me
  val goalFields = me.userGoalFields_user
  val goal = goalFields.goal
  val goalTarget = goal?.targetCharacterCount?.toLong()

  val today = remember { kstToday() }

  val history = remember(goalFields.goalHistory) { goalFields.goalHistory.toUserGoalDays() }
  val additions =
    remember(me.characterCountChanges) { me.characterCountChanges.toAdditionsByDate() }
  val achievements = remember(history) { history.toAchievementsByDate() }

  val dots = remember(history, today) { dotDays(history, today) }
  val trendRows =
    remember(additions, achievements, today) {
      dailyAdditionRows(additions, achievements, today, USER_GOAL_TREND_DAYS)
    }
  val historyRows =
    remember(additions, achievements, today) {
      dailyAdditionRows(additions, achievements, today, USER_GOAL_HISTORY_DAYS)
    }

  var targetInput by remember { mutableStateOf("") }
  var editing by remember { mutableStateOf(false) }
  var seeded by remember { mutableStateOf(false) }

  LaunchedEffect(loading, goal) {
    if (loading || seeded) {
      return@LaunchedEffect
    }

    seeded = true
    editing = false
    targetInput = goal?.targetCharacterCount?.toString() ?: ""
  }

  val save: suspend () -> Unit = save@{
    val target = targetInput.toGoalTargetOrNull()?.takeIf { it <= Int.MAX_VALUE }
    if (target == null) {
      toast.error("목표 글자 수를 올바르게 입력해 주세요.")
      return@save
    }

    model.save(target.toInt()).withDefaultExceptionHandler(toast).onOk {
      editing = false
      toast.success("일일 목표를 저장했어요.")
    }
  }

  val cancel: () -> Unit = {
    targetInput = goal?.targetCharacterCount?.toString() ?: ""
    editing = false
  }

  val remove: suspend () -> Unit = {
    val confirmation =
      dialog.confirm(
        title = "일일 목표를 해제하시겠어요?",
        message = "설정한 하루 목표 글자 수가 사라져요.",
        confirmText = "해제",
        confirmIsDestructive = true,
      )

    if (confirmation is DialogResult.Resolved) {
      model.delete().withDefaultExceptionHandler(toast).onOk {
        targetInput = ""
        toast.success("일일 목표를 해제했어요.")
      }
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton(icon = Lucide.X) },
    center = { Text(text = "일일 목표", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query) { innerPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .imePadding()
          .padding(innerPadding)
          .padding(AppTheme.spacings.scrollBottomPadding)
    ) {
      if (goalTarget != null && !editing) {
        UserGoalHero(
          history = history,
          target = goalTarget,
          today = today,
          onEdit = { editing = true },
          onRemove = remove,
        )
      } else {
        UserGoalForm(
          targetInput = targetInput,
          onTargetInputChange = { targetInput = it.filterGoalDigits() },
          showOnboarding = goal == null,
          cancellable = goal != null,
          onSave = save,
          onCancel = cancel,
        )
      }

      Spacer(Modifier.height(SectionGap))

      GoalSection(label = "달성 · 최근 16주") { UserGoalDots(days = dots) }

      Spacer(Modifier.height(SectionGap))

      GoalSection(label = "일별 글자 수 · 최근 4주") {
        UserGoalBarChart(rows = trendRows, target = goalTarget)
      }

      Spacer(Modifier.height(SectionGap))

      GoalSection(label = "일별 기록") { UserGoalHistoryTable(rows = historyRows) }
    }

    ToastAnchor(modifier = Modifier.align(Alignment.BottomCenter).navigationBarsPadding())
  }
}

@Composable
private fun UserGoalHero(
  history: List<UserGoalDay>,
  target: Long,
  today: LocalDate,
  onEdit: () -> Unit,
  onRemove: suspend () -> Unit,
) {
  val progress = remember(history, today) { todayProgress(history, today) }
  val streak = remember(history, today) { streaks(history, today) }

  Column(
    modifier = Modifier.fillMaxWidth(),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.spacedBy(HeroGap),
  ) {
    ProgressRing(
      progress = progress.additions.toFloat() / target.toFloat(),
      state = if (progress.achieved) GoalColorState.Achieved else GoalColorState.Under,
      size = RingSize,
    )

    Column(
      horizontalAlignment = Alignment.CenterHorizontally,
      verticalArrangement = Arrangement.spacedBy(HeroTextGap),
    ) {
      Text(
        text = progress.additions.comma,
        style = goalHeroNumberStyle,
        color = if (progress.achieved) AppTheme.colors.success else AppTheme.colors.textDefault,
      )

      Text(
        text = "/ ${target.comma}자",
        style = AppTheme.typography.caption.copy(fontFeatureSettings = TABULAR_FIGURES),
        color = AppTheme.colors.textMuted,
      )
    }

    Column(
      modifier =
        Modifier.fillMaxWidth()
          .background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.md))
          .padding(horizontal = ChipHorizontalPadding, vertical = ChipVerticalPadding),
      horizontalAlignment = Alignment.CenterHorizontally,
      verticalArrangement = Arrangement.spacedBy(ChipGap),
    ) {
      Text(
        text = "달성 연속 ${streak.current}일",
        style = AppTheme.typography.label,
        color = AppTheme.colors.textDefault,
      )

      Text(
        text = "최고 기록 ${streak.best}일",
        style = AppTheme.typography.micro,
        color = AppTheme.colors.textHint,
      )
    }

    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.spacedBy(ActionGap),
    ) {
      Button(
        text = "수정",
        variant = ButtonVariant.Secondary,
        modifier = Modifier.weight(1f),
        onClick = { onEdit() },
      )

      Button(
        text = "해제",
        variant = ButtonVariant.Danger,
        modifier = Modifier.weight(1f),
        onClick = onRemove,
      )
    }
  }
}

@Composable
private fun UserGoalForm(
  targetInput: String,
  onTargetInputChange: (String) -> Unit,
  showOnboarding: Boolean,
  cancellable: Boolean,
  onSave: suspend () -> Unit,
  onCancel: () -> Unit,
) {
  val commaTransformation = remember { CommaVisualTransformation() }

  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(FormGap),
  ) {
    if (showOnboarding) {
      Text(
        text = "매일 쓸 글자 수를 정해 보세요. 달성한 날이 기록으로 쌓이고, 연속 달성 일수도 볼 수 있어요.",
        modifier =
          Modifier.padding(bottom = OnboardingBottomMargin)
            .fillMaxWidth()
            .background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.md))
            .padding(OnboardingPadding),
        style = AppTheme.typography.caption.copy(lineHeight = OnboardingLineHeight),
        color = AppTheme.colors.textMuted,
      )
    }

    TextField(
      value = targetInput,
      onValueChange = onTargetInputChange,
      label = "하루 글자 수",
      labelPosition = LabelPosition.None,
      placeholder = "하루 글자 수",
      keyboardType = KeyboardType.Number,
      visualTransformation = commaTransformation,
      suffix = {
        Text(text = "자", style = AppTheme.typography.caption, color = AppTheme.colors.textHint)
      },
    )

    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.spacedBy(ActionGap),
    ) {
      if (cancellable) {
        Button(
          text = "취소",
          variant = ButtonVariant.Secondary,
          modifier = Modifier.weight(1f),
          onClick = { onCancel() },
        )
      }

      Button(text = "저장", modifier = Modifier.weight(1f), onClick = onSave)
    }
  }
}

private const val TABULAR_FIGURES = "tnum"

private val RingSize = 96.dp
private val SectionGap = 24.dp
private val HeroGap = 12.dp
private val HeroTextGap = 4.dp
private val ChipHorizontalPadding = 12.dp
private val ChipVerticalPadding = 8.dp
private val ChipGap = 2.dp
private val ActionGap = 12.dp
private val FormGap = 8.dp
private val OnboardingPadding = 12.dp
private val OnboardingBottomMargin = 4.dp
private val OnboardingLineHeight = 21.sp
