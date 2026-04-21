package co.typie.screen.more.stats

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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.FirstBaseline
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import co.typie.domain.stats.ActivityGrid
import co.typie.ext.comma
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.platform.FileSystemSaveLocation
import co.typie.platform.FileSystemSaveResult
import co.typie.platform.PlatformModule
import co.typie.result.onOk
import co.typie.result.withDefaultExceptionHandler
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.popover.PopoverMenu
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch

@Composable
fun StatsScreen() {
  val model = viewModel { StatsViewModel() }
  val toast = LocalToast.current

  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  ProvideTopBar(
    center = { Text("나의 글쓰기 통계", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(loadable = model.query) { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
      verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
      Text(
        "나의 글쓰기 통계",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      SummaryCard(
        label = "총 글자",
        value = model.query.data.me.usage.totalCharacterCount.comma,
        unit = "자",
      )

      Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        SummaryCard(
          label = "총 문서",
          value = model.query.data.me.documentCount.toString(),
          unit = "개",
          modifier = Modifier.weight(1f),
        )
        SummaryCard(
          label = "활동일",
          value = model.activity.totalActiveDays.toString(),
          unit = "일",
          modifier = Modifier.weight(1f),
        )
      }

      StreakCard(activity = model.activity)

      WeekdayCard(activity = model.activity)

      CardSurface(modifier = Modifier.fillMaxWidth(), clipContent = false) {
        Column(modifier = Modifier.fillMaxWidth()) {
          Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            Column(modifier = Modifier.weight(1f)) {
              Text(
                "지난 1년간의 기록",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textMuted,
              )
            }

            PopoverMenu(
              anchor = {
                Box(
                  modifier =
                    Modifier.border(
                        1.dp,
                        AppTheme.colors.borderEmphasis,
                        AppShapes.rounded(AppShapes.md),
                      )
                      .background(AppTheme.colors.surfaceDefault, AppShapes.rounded(AppShapes.md))
                      .padding(horizontal = 12.dp, vertical = 8.dp)
                ) {
                  Text(
                    "이미지 받기",
                    style = AppTheme.typography.action,
                    color = AppTheme.colors.textMuted,
                  )
                }
              },
              collapsedCornerRadius = AppShapes.md,
            ) {
              item(icon = Lucide.Copy, label = "클립보드에 복사") {
                scope.launch {
                  model.generateActivityImage().withDefaultExceptionHandler(toast).onOk {
                    val copied = PlatformModule.clipboard.copy(bytes = it, mimeType = "image/png")
                    if (copied) {
                      toast.success("이미지가 클립보드에 복사되었어요.")
                    } else {
                      toast.error("이미지를 복사할 수 없어요.")
                    }
                  }
                }
              }
              item(icon = Lucide.Download, label = "기기에 저장") {
                scope.launch {
                  model.generateActivityImage().withDefaultExceptionHandler(toast).onOk {
                    when (
                      PlatformModule.fileSystem.save(
                        bytes = it,
                        name = "${model.query.data.me.name}-나의-글쓰기-발자취.png",
                        location = FileSystemSaveLocation.Gallery,
                      )
                    ) {
                      FileSystemSaveResult.Success -> toast.success("이미지가 기기에 저장되었어요.")
                      FileSystemSaveResult.PermissionDenied -> toast.error("사진 접근 권한이 필요해요.")
                      FileSystemSaveResult.Error -> toast.error("이미지를 저장할 수 없어요.")
                    }
                  }
                }
              }
            }
          }

          ActivityGrid(
            user = model.query.data.me.activityGrid_user,
            modifier = Modifier.fillMaxWidth(),
          )

          Spacer(Modifier.height(16.dp))
        }
      }
    }
  }
}

@Composable
private fun SummaryCard(label: String, value: String, unit: String, modifier: Modifier = Modifier) {
  CardSurface(modifier = modifier.fillMaxWidth()) {
    Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
      Text(label, style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

      Spacer(Modifier.height(8.dp))

      Row(verticalAlignment = Alignment.Bottom) {
        Text(
          value,
          style = AppTheme.typography.heading,
          color = AppTheme.colors.textDefault,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Spacer(Modifier.width(4.dp))

        Text(
          unit,
          style = AppTheme.typography.action,
          color = AppTheme.colors.textMuted,
          modifier = Modifier.alignBy(FirstBaseline),
        )
      }
    }
  }
}

@Composable
private fun StreakCard(activity: ActivityData) {
  CardSurface(modifier = Modifier.fillMaxWidth()) {
    Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
      Text("연속 기록", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

      Spacer(Modifier.height(8.dp))

      Row(verticalAlignment = Alignment.Bottom) {
        Text(
          activity.currentStreak.toString(),
          style = AppTheme.typography.display,
          color = AppTheme.colors.textDefault,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Spacer(Modifier.width(4.dp))

        Text(
          "일째",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textMuted,
          modifier = Modifier.alignBy(FirstBaseline),
        )
      }
      Spacer(Modifier.height(12.dp))

      CardDivider(inset = 0.dp, color = AppTheme.colors.borderHairline)

      Spacer(Modifier.height(12.dp))

      Row(modifier = Modifier.fillMaxWidth()) {
        Text(
          "최장 ",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Text(
          "${activity.longestStreak}일",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textMuted,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Spacer(Modifier.width(12.dp))

        Text(
          "이번 달 ",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textMuted,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Text(
          "${activity.thisMonthActiveDays}일",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textMuted,
          modifier = Modifier.alignBy(FirstBaseline),
        )
      }
    }
  }
}

@Composable
private fun WeekdayCard(activity: ActivityData) {
  CardSurface(modifier = Modifier.fillMaxWidth()) {
    Column(modifier = Modifier.fillMaxWidth().padding(16.dp)) {
      Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text("요일별 기록", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

        Spacer(Modifier.weight(1f))

        if (activity.mostActiveWeekdayIndex != null) {
          Text(
            "${WeekdayNames[activity.mostActiveWeekdayIndex]}요일 최다",
            style = AppTheme.typography.micro,
            color = AppTheme.colors.textMuted,
          )
        }
      }

      Spacer(Modifier.height(16.dp))

      Row(
        modifier = Modifier.fillMaxWidth().height(52.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.Bottom,
      ) {
        val maxAverageAdditions =
          activity.weekdayActivities.maxOf { it.averageAdditions }.coerceAtLeast(1)
        for (weekdayActivity in activity.weekdayActivities) {
          val isMostActiveWeekday = weekdayActivity.dayIndex == activity.mostActiveWeekdayIndex
          val barHeight =
            (weekdayActivity.averageAdditions / maxAverageAdditions) * 32f.coerceAtLeast(2f)

          Column(
            modifier = Modifier.weight(1f),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Bottom,
          ) {
            Spacer(Modifier.weight(1f))

            Box(
              modifier =
                Modifier.fillMaxWidth()
                  .height(barHeight.dp)
                  .clip(AppShapes.rounded(AppShapes.sm))
                  .background(
                    if (isMostActiveWeekday) AppTheme.colors.textDefault
                    else AppTheme.colors.borderEmphasis
                  )
            )

            Spacer(Modifier.height(6.dp))

            Text(
              WeekdayNames[weekdayActivity.dayIndex],
              style = AppTheme.typography.micro,
              color =
                if (isMostActiveWeekday) AppTheme.colors.textDefault else AppTheme.colors.textMuted,
            )
          }
        }
      }
    }
  }
}

val WeekdayNames = listOf("일", "월", "화", "수", "목", "금", "토")
