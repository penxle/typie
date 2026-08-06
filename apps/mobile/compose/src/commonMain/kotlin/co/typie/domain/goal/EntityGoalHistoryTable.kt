package co.typie.domain.goal

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.ext.comma
import co.typie.ui.component.Divider
import co.typie.ui.component.Text
import co.typie.ui.theme.AppTheme
import kotlinx.datetime.LocalDate

const val GOAL_HISTORY_ROW_LIMIT: Int = 15

data class GoalHistoryRow(val date: LocalDate, val characterCount: Long, val diff: Long?)

fun goalHistoryRows(history: List<CharacterCountPoint>): List<GoalHistoryRow> {
  val recent = history.takeLast(GOAL_HISTORY_ROW_LIMIT).reversed()
  val rows = recent.mapIndexed { index, point ->
    GoalHistoryRow(
      date = point.date,
      characterCount = point.characterCount,
      diff =
        if (index < recent.lastIndex) {
          point.characterCount - recent[index + 1].characterCount
        } else {
          null
        },
    )
  }

  return if (history.size > 1) rows.dropLast(1) else rows
}

fun goalHistoryDiffLabel(diff: Long?): String =
  if (diff == null) "—" else "${if (diff >= 0) "+" else ""}${diff.comma}"

@Composable
fun EntityGoalHistoryTable(history: List<CharacterCountPoint>, modifier: Modifier = Modifier) {
  val rows = remember(history) { goalHistoryRows(history) }

  if (rows.isEmpty()) {
    GoalHistoryEmptyMessage(modifier)
    return
  }

  val headerStyle = AppTheme.typography.micro
  val rowStyle = AppTheme.typography.caption.copy(fontFeatureSettings = TABULAR_FIGURES)

  Column(modifier = modifier.fillMaxWidth()) {
    Row(modifier = Modifier.fillMaxWidth().padding(vertical = GoalHistoryRowPadding)) {
      GoalHistoryCell("날짜", TextAlign.Start, headerStyle, AppTheme.colors.textHint)
      GoalHistoryCell("글자 수", TextAlign.Center, headerStyle, AppTheme.colors.textHint)
      GoalHistoryCell("증감", TextAlign.End, headerStyle, AppTheme.colors.textHint)
    }

    Divider()

    rows.forEach { row ->
      Row(modifier = Modifier.fillMaxWidth().padding(vertical = GoalHistoryRowPadding)) {
        GoalHistoryCell(
          text = goalMonthDayLabel(row.date),
          textAlign = TextAlign.Start,
          style = rowStyle,
          color = AppTheme.colors.textHint,
        )

        GoalHistoryCell(
          text = "${row.characterCount.comma}자",
          textAlign = TextAlign.Center,
          style = rowStyle,
          color = AppTheme.colors.textMuted,
        )

        GoalHistoryCell(
          text = goalHistoryDiffLabel(row.diff),
          textAlign = TextAlign.End,
          style = rowStyle,
          color =
            if (row.diff != null && row.diff < 0) {
              AppTheme.colors.danger
            } else {
              AppTheme.colors.textHint
            },
        )
      }

      Divider()
    }
  }
}

@Composable
internal fun GoalHistoryEmptyMessage(modifier: Modifier = Modifier) {
  Text(
    text = "아직 기록이 없어요. 글을 쓰면 하루하루의 글자 수가 쌓여요.",
    modifier = modifier.fillMaxWidth().padding(vertical = GoalHistoryEmptyPadding),
    style = AppTheme.typography.caption,
    color = AppTheme.colors.textHint,
    textAlign = TextAlign.Center,
  )
}

@Composable
private fun RowScope.GoalHistoryCell(
  text: String,
  textAlign: TextAlign,
  style: TextStyle,
  color: Color,
) {
  Text(
    text = text,
    modifier = Modifier.weight(1f),
    style = style,
    color = color,
    overflow = TextOverflow.Ellipsis,
    maxLines = 1,
    textAlign = textAlign,
  )
}

private const val TABULAR_FIGURES = "tnum"

private val GoalHistoryRowPadding = 4.dp
private val GoalHistoryEmptyPadding = 24.dp
