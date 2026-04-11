package co.typie.screen.settings.editor_settings

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.SettingControlRow
import co.typie.ui.component.SettingSwitch
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.TopBarBackButton
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

@Composable
fun EditorSettingsScreen() {
  val scrollState = rememberScrollState()

  val typewriterEnabled = Preference.typewriterEnabled
  val typewriterPosition = Preference.typewriterPosition
  val lineHighlightEnabled = Preference.lineHighlightEnabled
  val autoSurroundEnabled = Preference.autoSurroundEnabled
  // TODO: 에디터 설정 트래킹

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("에디터", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(
    scrollState = scrollState,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
    Text("에디터", style = AppTheme.typography.display, modifier = Modifier.padding(top = 4.dp))

    EditorSettingsSection(title = "작성 위치") {
      SettingControlRow(
        label = "타자기 모드",
        description = "현재 작성 중인 줄을 항상 화면의 특정 위치에 고정합니다.",
        onClick = { Preference.typewriterEnabled = !typewriterEnabled },
        trailing = {
          SettingSwitch(
            checked = typewriterEnabled,
            onCheckedChange = { next -> Preference.typewriterEnabled = next },
          )
        },
      )

      if (typewriterEnabled) {
        CardDivider(inset = 20.dp)
        Column(
          modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 18.dp),
          verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
          Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text("고정 위치", style = AppTheme.typography.label)
            Text(
              "현재 작성 중인 줄이 고정될 화면상의 위치를 설정합니다.",
              style = AppTheme.typography.caption,
              color = AppTheme.colors.textTertiary,
            )
          }

          Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
          ) {
            Text("화면 상단", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
            QuietSlider(
              value = typewriterPosition,
              modifier = Modifier.weight(1f),
              onValueChange = { next -> Preference.typewriterPosition = next },
            )
            Text("화면 하단", style = AppTheme.typography.caption, color = AppTheme.colors.textTertiary)
          }
        }
      }
    }

    EditorSettingsSection(title = "표시 설정") {
      SettingControlRow(
        label = "현재 줄 강조",
        description = "현재 작성 중인 줄을 강조하여 화면에 표시합니다.",
        onClick = { Preference.lineHighlightEnabled = !lineHighlightEnabled },
        trailing = {
          SettingSwitch(
            checked = lineHighlightEnabled,
            onCheckedChange = { next -> Preference.lineHighlightEnabled = next },
          )
        },
      )
    }

    EditorSettingsSection(title = "편집 설정") {
      SettingControlRow(
        label = "선택 영역 둘러싸기",
        description = "따옴표나 괄호를 입력하면 선택 영역을 둘러쌉니다.",
        onClick = { Preference.autoSurroundEnabled = !autoSurroundEnabled },
        trailing = {
          SettingSwitch(
            checked = autoSurroundEnabled,
            onCheckedChange = { next -> Preference.autoSurroundEnabled = next },
          )
        },
      )
    }

    Spacer(Modifier.height(72.dp))
  }
}

@Composable
private fun EditorSettingsSection(title: String, content: @Composable ColumnScope.() -> Unit) {
  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    SectionTitle(title, modifier = Modifier.padding(top = 4.dp))

    CardSurface(modifier = Modifier.fillMaxWidth()) { Column(content = content) }
  }
}

@Composable
private fun QuietSlider(
  value: Double,
  onValueChange: (Double) -> Unit,
  modifier: Modifier = Modifier,
) {
  val colors = AppTheme.colors
  val haptic = LocalHapticFeedback.current
  BoxWithConstraints(modifier = modifier.height(32.dp), contentAlignment = Alignment.CenterStart) {
    val density = LocalDensity.current
    val thumbSize = 24.dp
    val travel = (maxWidth - thumbSize).coerceAtLeast(0.dp)
    val travelPx = with(density) { travel.toPx() }
    val sliderWidthPx = with(density) { maxWidth.toPx() }
    val thumbRadiusPx = with(density) { (thumbSize / 2).toPx() }
    val onValueChangeState = rememberUpdatedState(onValueChange)
    val hapticState = rememberUpdatedState(haptic)
    val thumbOffset = travel * value.toFloat().coerceIn(0f, 1f)
    val filledFraction = value.toFloat().coerceIn(0f, 1f)

    fun snap(raw: Float): Double {
      val stepped = (raw.coerceIn(0f, 1f) / 0.05f).roundToInt() * 0.05f
      return stepped.coerceIn(0f, 1f).toDouble()
    }

    fun valueAtTouch(x: Float): Double {
      if (travelPx <= 0f || sliderWidthPx <= 0f) return 0.0
      val fraction = ((x - thumbRadiusPx) / travelPx).coerceIn(0f, 1f)
      return snap(fraction)
    }

    Box(
      modifier =
        Modifier.fillMaxWidth()
          .height(8.dp)
          .background(colors.borderStrong.copy(alpha = 0.55f), CircleShape)
    ) {
      Box(
        modifier =
          Modifier.fillMaxWidth(filledFraction)
            .height(8.dp)
            .background(colors.brand.copy(alpha = 0.72f), CircleShape)
      )
    }

    Box(
      modifier =
        Modifier.matchParentSize().pointerInput(maxWidth) {
          awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false)
            var gestureValue = value

            fun updateGestureValue(x: Float) {
              val next = valueAtTouch(x)
              if (next == gestureValue) return
              gestureValue = next
              hapticState.value.performHapticFeedback(HapticFeedbackType.SegmentTick)
              onValueChangeState.value(next)
            }

            updateGestureValue(down.position.x)

            while (true) {
              val event = awaitPointerEvent()
              val change = event.changes.firstOrNull { it.id == down.id } ?: break
              if (!change.pressed) {
                break
              }
              updateGestureValue(change.position.x)
              change.consume()
            }
          }
        }
    )

    Box(
      modifier =
        Modifier.offset(x = thumbOffset)
          .size(thumbSize)
          .dropShadow(CircleShape) {
            color = colors.shadowAmbient
            radius = 4f
          }
          .dropShadow(CircleShape) {
            color = colors.shadow
            radius = 8f
            offset = Offset(0f, 1f)
          }
          .background(colors.surfaceRaised, CircleShape)
          .border(1.dp, colors.borderDefault, CircleShape)
    )
  }
}
