package co.typie.domain.goal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.datetime.kstToday
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.comma
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.ProgressBar
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme

fun goalMetricLabel(source: GoalSource?): String = if (source?.isFolder == true) "폴더 목표" else "목표"

fun goalMetricValue(source: GoalSource?): String {
  if (source == null) {
    return "설정 안 함"
  }

  val target = source.goal.targetCharacterCount
  val percent = (source.current.toDouble() / target * PERCENT_SCALE).toInt()

  return "${source.current.comma} / ${target.comma}자 ($percent%)"
}

@Composable
fun GoalMetricContent(source: GoalSource?, onOpenGoal: suspend () -> Unit) {
  if (source == null) {
    GoalMetricActionRow(label = "목표 설정", onClick = onOpenGoal)
    return
  }

  val today = remember { kstToday() }
  val target = source.goal.targetCharacterCount
  val status =
    source.goal.dueDate?.let { dueDate ->
      dueStatus(source.current, target, dueDate, today, DueStatusVariant.Compact)
    }

  Column(verticalArrangement = Arrangement.spacedBy(ContentGap)) {
    ProgressBar(
      progress = source.current.toFloat() / target.toFloat(),
      state = goalColorState(source.current, target),
    )

    Row(
      modifier = Modifier.fillMaxWidth(),
      horizontalArrangement = Arrangement.spacedBy(ValueGap),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Text(
        text = "${source.current.comma} / ${target.comma}자",
        modifier = Modifier.weight(1f),
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textMuted,
      )

      if (status != null) {
        Text(
          text = status.label,
          style = AppTheme.typography.caption,
          color = if (status.warning) AppTheme.colors.danger else AppTheme.colors.textHint,
        )
      }
    }

    GoalMetricActionRow(label = "목표 관리", onClick = onOpenGoal)
  }
}

@Composable
private fun GoalMetricActionRow(label: String, onClick: suspend () -> Unit) {
  InteractionScope {
    Row(
      modifier =
        Modifier.fillMaxWidth()
          .clickable(onClick = onClick)
          .pressScale()
          .padding(vertical = ActionRowVerticalPadding),
      horizontalArrangement = Arrangement.spacedBy(ValueGap),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Text(
        text = label,
        modifier = Modifier.weight(1f),
        style = AppTheme.typography.action,
        color = AppTheme.colors.textDefault,
      )

      Icon(
        icon = Lucide.ChevronRight,
        modifier = Modifier.size(ChevronSize),
        tint = AppTheme.colors.textHint,
      )
    }
  }
}

private const val PERCENT_SCALE = 100

private val ContentGap = 8.dp
private val ValueGap = 16.dp
private val ActionRowVerticalPadding = 12.dp
private val ChevronSize = 15.dp
