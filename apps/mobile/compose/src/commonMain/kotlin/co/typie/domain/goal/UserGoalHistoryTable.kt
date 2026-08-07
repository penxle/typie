package co.typie.domain.goal

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
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

const val USER_GOAL_HISTORY_DAYS: Int = 30

fun goalAchievementLabel(achieved: Boolean?): String =
  when (achieved) {
    true -> "달성"
    false -> "미달성"
    null -> "—"
  }

@Composable
fun UserGoalHistoryTable(rows: List<DailyAdditionRow>, modifier: Modifier = Modifier) {
  val headerStyle = AppTheme.typography.micro
  val rowStyle = AppTheme.typography.caption.copy(fontFeatureSettings = TABULAR_FIGURES)

  Column(modifier = modifier.fillMaxWidth()) {
    Row(modifier = Modifier.fillMaxWidth().padding(vertical = UserGoalHistoryRowPadding)) {
      UserGoalHistoryCell("날짜", TextAlign.Start, headerStyle, AppTheme.colors.textHint)
      UserGoalHistoryCell("쓴 글자", TextAlign.Center, headerStyle, AppTheme.colors.textHint)
      UserGoalHistoryCell("달성", TextAlign.End, headerStyle, AppTheme.colors.textHint)
    }

    Divider()

    rows.forEach { row ->
      Row(modifier = Modifier.fillMaxWidth().padding(vertical = UserGoalHistoryRowPadding)) {
        UserGoalHistoryCell(
          text = goalMonthDayLabel(row.date),
          textAlign = TextAlign.Start,
          style = rowStyle,
          color = AppTheme.colors.textHint,
        )

        UserGoalHistoryCell(
          text = "${row.additions.comma}자",
          textAlign = TextAlign.Center,
          style = rowStyle,
          color = AppTheme.colors.textMuted,
        )

        UserGoalHistoryCell(
          text = goalAchievementLabel(row.achieved),
          textAlign = TextAlign.End,
          style = rowStyle,
          color =
            when (row.achieved) {
              true -> AppTheme.colors.success
              false -> AppTheme.colors.textMuted
              null -> AppTheme.colors.textHint
            },
        )
      }

      Divider()
    }
  }
}

@Composable
private fun RowScope.UserGoalHistoryCell(
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

private val UserGoalHistoryRowPadding = 4.dp
