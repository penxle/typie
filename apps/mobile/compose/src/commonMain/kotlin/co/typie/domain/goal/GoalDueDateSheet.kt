package co.typie.domain.goal

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.datetime.WeekdayNames
import co.typie.ext.InteractionScope
import co.typie.ext.clickable
import co.typie.ext.pressScale
import co.typie.ext.thenIf
import co.typie.icons.Lucide
import co.typie.ui.component.Button
import co.typie.ui.component.ButtonVariant
import co.typie.ui.component.Text
import co.typie.ui.component.sheet.SheetBar
import co.typie.ui.component.sheet.SheetBarButton
import co.typie.ui.component.sheet.SheetBarDefaults
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetScope
import co.typie.ui.component.sheet.complete
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.datetime.DateTimeUnit
import kotlinx.datetime.LocalDate
import kotlinx.datetime.minus
import kotlinx.datetime.number
import kotlinx.datetime.plus

private const val GOAL_CALENDAR_SUNDAY_INDEX = 0
private const val GOAL_CALENDAR_SATURDAY_INDEX = 6
private val GoalCalendarRowSpacing = 2.dp
private val GoalCalendarCellInset = 4.dp
private val GoalCalendarTitleHorizontalPadding = 12.dp

sealed interface GoalDueDateResult {
  data class Selected(val date: LocalDate) : GoalDueDateResult

  data object Cleared : GoalDueDateResult
}

@Composable
context(_: SheetScope<GoalDueDateResult>)
fun GoalDueDateSheet(initial: LocalDate?, today: LocalDate) {
  val base = initial ?: today
  var viewMonth by remember { mutableStateOf(LocalDate(base.year, base.month.number, 1)) }
  val weeks = remember(viewMonth) { monthGridWeeks(viewMonth.year, viewMonth.month.number) }

  val footer: (@Composable ColumnScope.() -> Unit)? =
    if (initial == null) {
      null
    } else {
      {
        Button(
          text = "마감일 제거",
          variant = ButtonVariant.Secondary,
          onClick = { complete(GoalDueDateResult.Cleared) },
        )
      }
    }

  SheetLayout(
    padding = SheetPadding(body = PaddingValues(0.dp)),
    header = {
      SheetBar(
        leading = {
          SheetBarButton(
            icon = Lucide.ChevronLeft,
            onClick = { viewMonth = viewMonth.minus(1, DateTimeUnit.MONTH) },
          )
        },
        center = {
          InteractionScope {
            Box(
              modifier =
                Modifier.defaultMinSize(minHeight = SheetBarDefaults.SlotWidth)
                  .clickable { viewMonth = LocalDate(today.year, today.month.number, 1) }
                  .pressScale(0.96f)
                  .padding(horizontal = GoalCalendarTitleHorizontalPadding),
              contentAlignment = Alignment.Center,
            ) {
              Text(
                text = "${viewMonth.year}년 ${viewMonth.month.number}월",
                style = AppTheme.typography.title,
                color = AppTheme.colors.textDefault,
                overflow = TextOverflow.Ellipsis,
                maxLines = 1,
              )
            }
          }
        },
        trailing = {
          SheetBarButton(
            icon = Lucide.ChevronRight,
            onClick = { viewMonth = viewMonth.plus(1, DateTimeUnit.MONTH) },
          )
        },
      )
    },
    footer = footer,
  ) {
    BoxWithConstraints(modifier = Modifier.fillMaxWidth()) {
      Column(
        modifier = Modifier.padding(horizontal = goalCalendarHorizontalPadding(maxWidth)),
        verticalArrangement = Arrangement.spacedBy(GoalCalendarRowSpacing),
      ) {
        Row(modifier = Modifier.fillMaxWidth()) {
          WeekdayNames.forEach { name ->
            Text(
              text = name,
              modifier = Modifier.weight(1f),
              style = AppTheme.typography.micro,
              color = AppTheme.colors.textHint,
              textAlign = TextAlign.Center,
            )
          }
        }

        weeks.forEach { week ->
          Row(modifier = Modifier.fillMaxWidth()) {
            week.forEachIndexed { index, date ->
              if (date == null) {
                GoalCalendarEmptyCell()
              } else {
                GoalCalendarDayCell(
                  day = date.day,
                  weekdayIndex = index,
                  selected = date == initial,
                  isToday = date == today,
                  modifier = Modifier.weight(1f),
                  onSelect = { complete(GoalDueDateResult.Selected(date)) },
                )
              }
            }
          }
        }
      }
    }
  }
}

@Composable
private fun weekdayColor(weekdayIndex: Int) =
  if (weekdayIndex == GOAL_CALENDAR_SUNDAY_INDEX || weekdayIndex == GOAL_CALENDAR_SATURDAY_INDEX) {
    AppTheme.colors.textHint
  } else {
    AppTheme.colors.textDefault
  }

@Composable
private fun RowScope.GoalCalendarEmptyCell() {
  Spacer(modifier = Modifier.weight(1f).aspectRatio(1f))
}

@Composable
private fun GoalCalendarDayCell(
  day: Int,
  weekdayIndex: Int,
  selected: Boolean,
  isToday: Boolean,
  modifier: Modifier = Modifier,
  onSelect: () -> Unit,
) {
  InteractionScope {
    Box(
      modifier = modifier.aspectRatio(1f).clickable { onSelect() }.pressScale(0.94f),
      contentAlignment = Alignment.Center,
    ) {
      Box(
        modifier =
          Modifier.fillMaxSize()
            .padding(GoalCalendarCellInset)
            .clip(AppShapes.circle)
            .thenIf(selected) { background(AppTheme.colors.surfaceInverse, AppShapes.circle) }
            .thenIf(isToday && !selected) {
              border(1.dp, AppTheme.colors.borderEmphasis, AppShapes.circle)
            },
        contentAlignment = Alignment.Center,
      ) {
        Text(
          text = day.toString(),
          style = AppTheme.typography.caption,
          color = if (selected) AppTheme.colors.textOnInverse else weekdayColor(weekdayIndex),
        )
      }
    }
  }
}
