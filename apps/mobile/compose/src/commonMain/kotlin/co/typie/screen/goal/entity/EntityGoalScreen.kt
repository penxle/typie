package co.typie.screen.goal.entity

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.datetime.toKstLocalDate
import co.typie.domain.goal.EntityGoalHistoryTable
import co.typie.domain.goal.EntityGoalTrendChart
import co.typie.domain.goal.GoalColorState
import co.typie.domain.goal.GoalDueDateResult
import co.typie.domain.goal.GoalDueDateSheet
import co.typie.domain.goal.GoalSection
import co.typie.domain.goal.dDayLabel
import co.typie.domain.goal.goalHeroNumberStyle
import co.typie.domain.goal.toEntityGoalData
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.comma
import co.typie.ext.navigationBarsPadding
import co.typie.ext.pressScale
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
import co.typie.ui.component.sheet.LocalSheet
import co.typie.ui.component.toGoalTargetOrNull
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.toast.ToastAnchor
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.skeleton.Skeleton
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.time.Clock
import kotlin.time.Instant
import kotlinx.datetime.LocalDate
import kotlinx.datetime.number

@Composable
fun EntityGoalScreen(entityId: String) {
  val model = viewModel { EntityGoalViewModel() }
  val toast = LocalToast.current
  val dialog = LocalDialog.current
  val sheet = LocalSheet.current
  val scrollState = rememberScrollState()

  LaunchedEffect(entityId) { model.entityId = entityId }

  val loading = model.query.state !is QueryState.Success
  val entity = model.query.data.entity
  val goal = remember(entity.goal) { entity.goal?.entityGoalFields_goal?.toEntityGoalData() }
  val history =
    remember(entity.characterCountHistory) { entity.characterCountHistory.toCharacterCountPoints() }
  val currentCount = entity.node.currentCharacterCount()
  val targetName = entity.node.targetName()

  val now = remember { Clock.System.now() }
  val today = remember(now) { now.toKstLocalDate() }

  var targetInput by remember(entityId) { mutableStateOf("") }
  var dueDate by remember(entityId) { mutableStateOf<LocalDate?>(null) }
  var editing by remember(entityId) { mutableStateOf(false) }
  var seeded by remember(entityId) { mutableStateOf(false) }

  LaunchedEffect(entityId, loading, goal) {
    if (loading || seeded) {
      return@LaunchedEffect
    }

    seeded = true
    editing = false
    targetInput = goal?.targetCharacterCount?.toString() ?: ""
    dueDate = goal?.dueDate
  }

  val save: suspend () -> Unit = save@{
    val target = targetInput.toGoalTargetOrNull()?.takeIf { it <= Int.MAX_VALUE }
    if (target == null) {
      toast.error("목표 글자 수를 올바르게 입력해 주세요.")
      return@save
    }

    model.save(target.toInt(), dueDate).withDefaultExceptionHandler(toast).onOk {
      editing = false
      toast.success("목표를 저장했어요.")
    }
  }

  val cancel: () -> Unit = {
    targetInput = goal?.targetCharacterCount?.toString() ?: ""
    dueDate = goal?.dueDate
    editing = false
  }

  val remove: suspend () -> Unit = {
    val confirmation =
      dialog.confirm(
        title = "목표를 삭제하시겠어요?",
        message = "설정한 목표 글자 수와 마감일이 사라져요.",
        confirmText = "삭제",
        confirmIsDestructive = true,
      )

    if (confirmation is DialogResult.Resolved) {
      model.delete().withDefaultExceptionHandler(toast).onOk {
        targetInput = ""
        dueDate = null
        toast.success("목표를 삭제했어요.")
      }
    }
  }

  val pickDueDate: suspend () -> Unit = {
    when (val result = sheet.present { GoalDueDateSheet(initial = dueDate, today = today) }) {
      is GoalDueDateResult.Selected -> dueDate = result.date
      GoalDueDateResult.Cleared -> dueDate = null
      null -> Unit
    }
  }

  ProvideTopBar(
    leading = { TopBarBackButton(icon = Lucide.X) },
    center = {
      Skeleton.Passive(enabled = loading) {
        Text(
          text = "목표 · $targetName",
          style = AppTheme.typography.title,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
        )
      }
    },
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
      if (goal != null && !editing) {
        EntityGoalHero(
          current = currentCount,
          target = goal.targetCharacterCount,
          createdDate = goal.createdDate,
          dueDate = goal.dueDate,
          today = today,
          now = now,
          onEdit = { editing = true },
          onDelete = remove,
        )
      } else {
        EntityGoalForm(
          targetInput = targetInput,
          onTargetInputChange = { targetInput = it.filterGoalDigits() },
          dueDate = dueDate,
          onPickDueDate = pickDueDate,
          showOnboarding = goal == null,
          cancellable = goal != null,
          onSave = save,
          onCancel = cancel,
        )
      }

      Spacer(Modifier.height(SectionGap))

      GoalSection(label = "추세") {
        EntityGoalTrendChart(history = history, current = currentCount, goal = goal, today = today)
      }

      Spacer(Modifier.height(SectionGap))

      GoalSection(label = "기록") { EntityGoalHistoryTable(history = history) }
    }

    ToastAnchor(modifier = Modifier.align(Alignment.BottomCenter).navigationBarsPadding())
  }
}

@Composable
private fun EntityGoalHero(
  current: Long,
  target: Long,
  createdDate: LocalDate,
  dueDate: LocalDate?,
  today: LocalDate,
  now: Instant,
  onEdit: () -> Unit,
  onDelete: suspend () -> Unit,
) {
  val hero =
    remember(current, target, createdDate, dueDate, today, now) {
      entityGoalHeroState(current, target, createdDate, dueDate, today, now)
    }

  Column(
    modifier = Modifier.fillMaxWidth(),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.spacedBy(HeroGap),
  ) {
    ProgressRing(
      progress = current.toFloat() / target.toFloat(),
      state = hero.colorState,
      pie = hero.pie,
      pieWarning = hero.overdueUnder,
      size = RingSize,
    )

    Column(
      horizontalAlignment = Alignment.CenterHorizontally,
      verticalArrangement = Arrangement.spacedBy(HeroTextGap),
    ) {
      Text(
        text = "${hero.percent}%",
        style = goalHeroNumberStyle,
        color = percentColor(hero.colorState),
      )

      Text(
        text = "${current.comma} / ${target.comma}자",
        style = AppTheme.typography.caption.copy(fontFeatureSettings = TABULAR_FIGURES),
        color = AppTheme.colors.textMuted,
      )

      EntityGoalDeltaText(
        state = hero.colorState,
        current = current,
        target = target,
        overdue = hero.overdue,
      )
    }

    if (dueDate != null && hero.dueChipVisible) {
      EntityGoalDueChip(
        dueDate = dueDate,
        today = today,
        overdueUnder = hero.overdueUnder,
        required = hero.required,
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
        text = "삭제",
        variant = ButtonVariant.Danger,
        modifier = Modifier.weight(1f),
        onClick = onDelete,
      )
    }
  }
}

@Composable
private fun EntityGoalDeltaText(
  state: GoalColorState,
  current: Long,
  target: Long,
  overdue: Boolean,
) {
  if (state == GoalColorState.Under && !overdue) {
    Text(
      text = "${(target - current).comma}자 남음",
      style = AppTheme.typography.micro,
      color = AppTheme.colors.textHint,
    )
  } else if (state == GoalColorState.Over || state == GoalColorState.Excess) {
    Text(
      text = "목표보다 ${(current - target).comma}자 초과",
      style = AppTheme.typography.micro,
      color =
        if (state == GoalColorState.Excess) AppTheme.colors.danger else AppTheme.colors.textHint,
    )
  }
}

@Composable
private fun EntityGoalDueChip(
  dueDate: LocalDate,
  today: LocalDate,
  overdueUnder: Boolean,
  required: Long,
) {
  Column(
    modifier =
      Modifier.fillMaxWidth()
        .background(AppTheme.colors.surfaceInset, AppShapes.rounded(AppShapes.md))
        .padding(horizontal = ChipHorizontalPadding, vertical = ChipVerticalPadding),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.spacedBy(ChipGap),
  ) {
    Text(
      text = dDayLabel(dueDate, today),
      style = AppTheme.typography.label,
      color = if (overdueUnder) AppTheme.colors.danger else AppTheme.colors.textDefault,
    )

    if (required > 0) {
      Text(
        text = if (overdueUnder) "${required.comma}자 남음" else "오늘 ${required.comma}자 필요",
        style = AppTheme.typography.micro,
        color = AppTheme.colors.textHint,
      )
    }
  }
}

@Composable
private fun EntityGoalForm(
  targetInput: String,
  onTargetInputChange: (String) -> Unit,
  dueDate: LocalDate?,
  onPickDueDate: suspend () -> Unit,
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
        text = "완성까지 쓸 글자 수를 목표로 정해 보세요. 마감일을 함께 정하면 오늘 써야 할 분량도 알려드려요.",
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
      label = "목표 글자 수",
      labelPosition = LabelPosition.None,
      placeholder = "목표 글자 수",
      keyboardType = KeyboardType.Number,
      visualTransformation = commaTransformation,
      suffix = {
        Text(text = "자", style = AppTheme.typography.caption, color = AppTheme.colors.textHint)
      },
    )

    GoalDueDateField(dueDate = dueDate, onClick = onPickDueDate)

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

@Composable
private fun GoalDueDateField(dueDate: LocalDate?, onClick: suspend () -> Unit) {
  val shape = AppShapes.rounded(AppShapes.md)

  InteractionScope {
    Box(
      modifier =
        Modifier.fillMaxWidth()
          .height(FieldHeight)
          .border(FieldBorderWidth, AppTheme.colors.borderHairline, shape)
          .background(AppTheme.colors.surfaceDefault, shape)
          .clickable(onClick = onClick)
          .pressScale()
          .padding(horizontal = FieldHorizontalPadding),
      contentAlignment = Alignment.CenterStart,
    ) {
      Text(
        text =
          if (dueDate == null) {
            "마감일 없음 (선택)"
          } else {
            "${dueDate.year}. ${dueDate.month.number}. ${dueDate.day}. 마감"
          },
        style = AppTheme.typography.body,
        color = if (dueDate == null) AppTheme.colors.textHint else AppTheme.colors.textDefault,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        textAlign = TextAlign.Start,
      )
    }
  }
}

@Composable
private fun percentColor(state: GoalColorState): Color =
  when (state) {
    GoalColorState.Under -> AppTheme.colors.textDefault
    GoalColorState.Achieved -> AppTheme.colors.success
    GoalColorState.Over -> AppTheme.colors.warning
    GoalColorState.Excess -> AppTheme.colors.danger
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
private val FieldHeight = 48.dp
private val FieldBorderWidth = 1.dp
private val FieldHorizontalPadding = 16.dp
