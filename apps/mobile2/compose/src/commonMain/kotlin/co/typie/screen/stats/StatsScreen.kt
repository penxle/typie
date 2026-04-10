@file:OptIn(kotlin.io.encoding.ExperimentalEncodingApi::class)

package co.typie.screen.stats

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
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.FirstBaseline
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import co.typie.datetime.toLocalDate
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.graphql.QueryState
import co.typie.graphql.StatsScreen_GenerateActivityImage_Mutation
import co.typie.graphql.executeMutation
import co.typie.icons.Lucide
import co.typie.overlay.LocalToast
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.platform.Clipboard
import co.typie.platform.FileSystem
import co.typie.platform.FileSystemSaveLocation
import co.typie.platform.FileSystemSaveResult
import co.typie.ui.component.ActivityGrid
import co.typie.ui.component.ActivityGridChange
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.ErrorDialog
import co.typie.ui.component.Screen
import co.typie.ui.component.Text
import co.typie.ui.component.popover.Popover
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppTheme
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.launch
import kotlinx.datetime.LocalDate
import kotlinx.datetime.number
import org.koin.compose.koinInject
import org.koin.compose.viewmodel.koinViewModel
import kotlin.io.encoding.Base64
import kotlin.math.max

@Composable
fun StatsScreen() {
  val model = koinViewModel<StatsViewModel>()
  val apolloClient = koinInject<ApolloClient>()
  val toast = LocalToast.current
  val clipboard = koinInject<Clipboard>()
  val fileSystem = koinInject<FileSystem>()
  val scrollState = rememberScrollState()
  val scope = rememberCoroutineScope()

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("나의 글쓰기 통계", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  if (model.query.state is QueryState.Error) {
    ErrorDialog { model.query.refetch() }
  }

  Screen(
    scrollState = scrollState,
    loading = model.query.state !is QueryState.Success,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    val data = model.query.data
    val changes = remember(data.me.characterCountChanges) {
      data.me.characterCountChanges.map { change ->
        StatsCharacterCountChange(
          date = change.date.toLocalDate(),
          additions = change.additions,
          deletions = change.deletions,
        )
      }
    }
    val totalCharacterCount = data.me.usage.totalCharacterCount
    val streakData = remember(changes, totalCharacterCount) {
      calculateStreakData(changes, totalCharacterCount)
    }
    val weekdayData = remember(changes) { calculateWeekdayPattern(changes) }
    val bestWeekday = remember(weekdayData) {
      weekdayData.maxByOrNull { it.avgAdditions }?.takeIf { it.avgAdditions > 0 }
    }

    suspend fun fetchActivityImage(): ByteArray? {
      return runCatching {
        apolloClient.executeMutation(StatsScreen_GenerateActivityImage_Mutation())
      }.mapCatching { result ->
        Base64.decode(result.generateActivityImage.toString())
      }.getOrNull()
    }

    fun copyActivityImage() {
      scope.launch {
        val bytes = fetchActivityImage()
        if (bytes == null) {
          toast.show(ToastType.Error, "이미지를 복사할 수 없어요.")
          return@launch
        }

        val copied = clipboard.copy(bytes = bytes, mimeType = "image/png")

        if (copied) {
          toast.show(ToastType.Success, "이미지가 클립보드에 복사되었어요.")
        } else {
          toast.show(ToastType.Error, "이미지를 복사할 수 없어요.")
        }
      }
    }

    fun saveActivityImage() {
      scope.launch {
        val bytes = fetchActivityImage()
        if (bytes == null) {
          toast.show(ToastType.Error, "이미지를 저장할 수 없어요.")
          return@launch
        }

        when (
          fileSystem.save(
            bytes = bytes,
            name = "${data.me.name}-나의-글쓰기-발자취.png",
            location = FileSystemSaveLocation.Gallery,
          )
        ) {
          FileSystemSaveResult.Success -> toast.show(ToastType.Success, "이미지가 기기에 저장되었어요.")
          FileSystemSaveResult.PermissionDenied -> toast.show(ToastType.Error, "사진 접근 권한이 필요해요.")
          FileSystemSaveResult.Error -> toast.show(ToastType.Error, "이미지를 저장할 수 없어요.")
        }
      }
    }

      Text(
        "나의 글쓰기 통계",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      SummaryCard(
        label = "총 글자",
        value = totalCharacterCount.formatGrouped(),
        unit = "자",
      )

      Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        SummaryCard(
          label = "총 문서",
          value = data.me.documentCount.toString(),
          unit = "개",
          modifier = Modifier.weight(1f),
        )
        SummaryCard(
          label = "활동일",
          value = streakData.totalDays.toString(),
          unit = "일",
          modifier = Modifier.weight(1f),
        )
      }

      StreakCard(streakData = streakData)

      WeekdayCard(
        weekdayData = weekdayData,
        bestWeekdayIndex = bestWeekday?.dayIndex ?: -1,
        maxWeekdayAverage = bestWeekday?.avgAdditions ?: 0,
      )

      CardSurface(
        modifier = Modifier.fillMaxWidth(),
        clipContent = false,
      ) {
        Column(
          modifier = Modifier.fillMaxWidth(),
        ) {
          Row(
            modifier = Modifier
              .fillMaxWidth()
              .padding(horizontal = 16.dp, vertical = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
          ) {
            Column(
              modifier = Modifier.weight(1f),
            ) {
              Text(
                "지난 1년간의 기록",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textSecondary,
              )
            }

            Popover(
              anchor = {
                StatsActionButton(label = "이미지 받기")
              },
              collapsedCornerRadius = 10.dp,
              pane = {
                Column(
                  modifier = Modifier.padding(PopoverDefaults.PanePadding),
                ) {
                  PopoverList(
                    items = listOf(
                      PopoverListItem(
                        content = {
                          StatsActionItem(
                            icon = Lucide.Copy,
                            label = "클립보드에 복사",
                          )
                        },
                        onSelected = {
                          close()
                          copyActivityImage()
                        },
                      ),
                      PopoverListItem(
                        content = {
                          StatsActionItem(
                            icon = Lucide.Download,
                            label = "기기에 저장",
                          )
                        },
                        onSelected = {
                          close()
                          saveActivityImage()
                        },
                      ),
                    ),
                  )
                }
              },
            )
          }

          ActivityGrid(
            changes = changes.map { change ->
              ActivityGridChange(
                date = change.date,
                additions = change.additions,
              )
            },
            modifier = Modifier.fillMaxWidth(),
            onVerticalScrollDelta = { delta -> scrollState.dispatchRawDelta(delta) },
          )
        }
      }

      CardSurface(
        modifier = Modifier.fillMaxWidth(),
        clipContent = false,
      ) {
        Box(
          modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 16.dp),
        ) {
          StatsActivityChart(
            characterCountChanges = changes,
            onVerticalScrollDelta = { delta -> scrollState.dispatchRawDelta(delta) },
          )
        }
      }

      Spacer(Modifier.height(140.dp))
  }
}

@Composable
private fun SummaryCard(
  label: String,
  value: String,
  unit: String,
  modifier: Modifier = Modifier,
) {
  CardSurface(
    modifier = modifier.fillMaxWidth(),
  ) {
    Column(
      modifier = Modifier
        .fillMaxWidth()
        .padding(16.dp),
    ) {
      Text(
        label,
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textSecondary,
      )

      Spacer(Modifier.height(8.dp))

      Row(
        verticalAlignment = Alignment.Bottom,
      ) {
        Text(
          value,
          style = AppTheme.typography.heading,
          color = AppTheme.colors.textPrimary,
          maxLines = 1,
          overflow = TextOverflow.Ellipsis,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Spacer(Modifier.width(4.dp))

        Text(
          unit,
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.alignBy(FirstBaseline),
        )
      }
    }
  }
}

@Composable
private fun StreakCard(
  streakData: StreakData,
) {
  CardSurface(
    modifier = Modifier.fillMaxWidth(),
  ) {
    Column(
      modifier = Modifier
        .fillMaxWidth()
        .padding(16.dp),
    ) {
      Text(
        "연속 기록",
        style = AppTheme.typography.caption,
        color = AppTheme.colors.textSecondary,
      )

      Spacer(Modifier.height(8.dp))

      Row(
        verticalAlignment = Alignment.Bottom,
      ) {
        Text(
          streakData.currentStreak.toString(),
          style = AppTheme.typography.display,
          color = AppTheme.colors.textPrimary,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Spacer(Modifier.width(4.dp))

        Text(
          "일째",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.alignBy(FirstBaseline),
        )
      }
      Spacer(Modifier.height(12.dp))

      CardDivider(
        inset = 0.dp,
        color = AppTheme.colors.borderSubtle,
      )

      Spacer(Modifier.height(12.dp))

      Row(
        modifier = Modifier.fillMaxWidth(),
      ) {
        Text(
          "최장 ",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Text(
          "${streakData.longestStreak}일",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textSecondary,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Spacer(Modifier.width(12.dp))

        Text(
          "이번 달 ",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textTertiary,
          modifier = Modifier.alignBy(FirstBaseline),
        )

        Text(
          "${streakData.thisMonthDays}일",
          style = AppTheme.typography.action,
          color = AppTheme.colors.textSecondary,
          modifier = Modifier.alignBy(FirstBaseline),
        )
      }
    }
  }
}

@Composable
private fun WeekdayCard(
  weekdayData: List<WeekdayData>,
  bestWeekdayIndex: Int,
  maxWeekdayAverage: Int,
) {
  CardSurface(
    modifier = Modifier.fillMaxWidth(),
  ) {
    Column(
      modifier = Modifier
        .fillMaxWidth()
        .padding(16.dp),
    ) {
      Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        Text(
          "요일별 기록",
          style = AppTheme.typography.caption,
          color = AppTheme.colors.textSecondary,
        )

        Spacer(Modifier.weight(1f))

        if (bestWeekdayIndex >= 0) {
          Text(
            "${weekdayLabels[bestWeekdayIndex]}요일 최다",
            style = AppTheme.typography.micro,
            color = AppTheme.colors.textTertiary,
          )
        }
      }

      Spacer(Modifier.height(16.dp))

      Row(
        modifier = Modifier
          .fillMaxWidth()
          .height(52.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.Bottom,
      ) {
        weekdayData.forEach { day ->
          val isBest = day.dayIndex == bestWeekdayIndex
          val safeMax = max(maxWeekdayAverage, 1)
          val barHeight = max((day.avgAdditions / safeMax.toFloat()) * 32f, 2f)

          Column(
            modifier = Modifier.weight(1f),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Bottom,
          ) {
            Spacer(Modifier.weight(1f))

            Box(
              modifier = Modifier
                .fillMaxWidth()
                .height(barHeight.dp)
                .clip(RoundedCornerShape(3.dp))
                .background(if (isBest) AppTheme.colors.textPrimary else AppTheme.colors.borderStrong),
            )

            Spacer(Modifier.height(6.dp))

            Text(
              day.label,
              style = AppTheme.typography.micro,
              color = if (isBest) AppTheme.colors.textPrimary else AppTheme.colors.textTertiary,
            )
          }
        }
      }
    }
  }
}

@Composable
private fun StatsActionButton(
  label: String,
) {
  Box(
    modifier = Modifier
      .clip(RoundedCornerShape(8.dp))
      .background(AppTheme.colors.surfaceDefault)
      .border(1.dp, AppTheme.colors.borderStrong, RoundedCornerShape(8.dp))
      .padding(horizontal = 12.dp, vertical = 7.dp),
  ) {
    Text(
      label,
      style = AppTheme.typography.action,
      color = AppTheme.colors.textSecondary,
    )
  }
}

@Composable
private fun StatsActionItem(
  icon: co.typie.ui.icon.IconData,
  label: String,
) {
  Row(
    modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
    verticalAlignment = Alignment.CenterVertically,
    horizontalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    Icon(
      icon = icon,
      modifier = Modifier.size(18.dp),
      tint = AppTheme.colors.textPrimary,
    )

    Text(
      label,
      style = AppTheme.typography.action,
      color = AppTheme.colors.textPrimary,
    )
  }
}

internal fun Int.formatGrouped(): String {
  val text = toString()
  val builder = StringBuilder()

  text.forEachIndexed { index, char ->
    if (index > 0 && (text.length - index) % 3 == 0) {
      builder.append(',')
    }
    builder.append(char)
  }

  return builder.toString()
}

internal fun formatFullDate(date: LocalDate): String {
  return "${date.year}년 ${date.month.number}월 ${date.day}일"
}
