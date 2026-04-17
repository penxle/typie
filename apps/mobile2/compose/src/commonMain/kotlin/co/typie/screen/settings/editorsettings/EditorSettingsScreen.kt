package co.typie.screen.settings.editorsettings

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.drag
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.times
import co.typie.domain.settings.SettingControlRow
import co.typie.domain.settings.SettingSwitch
import co.typie.ext.verticalScroll
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt

@Composable
fun EditorSettingsScreen() {
  val scrollState = rememberScrollState()

  ProvideTopBar(
    center = { Text("에디터", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen { contentPadding ->
    Column(
      modifier = Modifier.fillMaxSize().verticalScroll(scrollState).padding(contentPadding),
      verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
      Text("에디터", style = AppTheme.typography.display)

      EditorSettingsSection(title = "작성 위치") {
        SettingControlRow(
          label = "타자기 모드",
          description = "현재 작성 중인 줄을 항상 화면의 특정 위치에 고정합니다.",
          onClick = { Preference.typewriterEnabled = !Preference.typewriterEnabled },
          trailing = {
            SettingSwitch(
              checked = Preference.typewriterEnabled,
              onCheckedChange = { next -> Preference.typewriterEnabled = next },
            )
          },
        )

        AnimatedVisibility(visible = Preference.typewriterEnabled) {
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
              Text(
                "화면 상단",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )

              Slider(
                value = Preference.typewriterPosition,
                onValueChange = { next -> Preference.typewriterPosition = next },
                modifier = Modifier.weight(1f),
              )

              Text(
                "화면 하단",
                style = AppTheme.typography.caption,
                color = AppTheme.colors.textTertiary,
              )
            }
          }
        }
      }

      EditorSettingsSection(title = "표시 설정") {
        SettingControlRow(
          label = "현재 줄 강조",
          description = "현재 작성 중인 줄을 강조하여 화면에 표시합니다.",
          onClick = { Preference.lineHighlightEnabled = !Preference.lineHighlightEnabled },
          trailing = {
            SettingSwitch(
              checked = Preference.lineHighlightEnabled,
              onCheckedChange = { next -> Preference.lineHighlightEnabled = next },
            )
          },
        )
      }

      EditorSettingsSection(title = "편집 설정") {
        SettingControlRow(
          label = "선택 영역 둘러싸기",
          description = "따옴표나 괄호를 입력하면 선택 영역을 둘러쌉니다.",
          onClick = { Preference.autoSurroundEnabled = !Preference.autoSurroundEnabled },
          trailing = {
            SettingSwitch(
              checked = Preference.autoSurroundEnabled,
              onCheckedChange = { next -> Preference.autoSurroundEnabled = next },
            )
          },
        )
      }
    }
  }
}

@Composable
private fun EditorSettingsSection(title: String, content: @Composable ColumnScope.() -> Unit) {
  Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
    SectionTitle(title)

    CardSurface(modifier = Modifier.fillMaxWidth()) { Column(content = content) }
  }
}

@Composable
private fun Slider(value: Double, onValueChange: (Double) -> Unit, modifier: Modifier = Modifier) {
  val colors = AppTheme.colors

  val density = LocalDensity.current
  val haptic = LocalHapticFeedback.current

  val thumbSize = 24.dp

  BoxWithConstraints(modifier = modifier.height(32.dp), contentAlignment = Alignment.CenterStart) {
    val travel = (maxWidth - thumbSize).coerceAtLeast(0.dp)
    val travelPx = with(density) { travel.toPx() }
    val thumbRadiusPx = with(density) { (thumbSize / 2).toPx() }
    val filledFraction = value.toFloat().coerceIn(0f, 1f)
    val thumbOffset = filledFraction * travel

    fun coerceValue(x: Float): Double {
      val fraction = ((x - thumbRadiusPx) / travelPx).coerceIn(0f, 1f)
      val snapped = (fraction.coerceIn(0f, 1f) / 0.05f).roundToInt() * 0.05f
      return snapped.coerceIn(0f, 1f).toDouble()
    }

    Box(
      modifier =
        Modifier.fillMaxWidth()
          .height(8.dp)
          .background(AppTheme.colors.borderStrong.copy(alpha = 0.5f), AppShapes.circle)
    ) {
      Box(
        modifier =
          Modifier.fillMaxWidth(filledFraction)
            .height(8.dp)
            .background(AppTheme.colors.brand.copy(alpha = 0.75f), AppShapes.circle)
      )
    }

    Box(
      modifier =
        Modifier.matchParentSize().pointerInput(maxWidth) {
          awaitEachGesture {
            val down = awaitFirstDown(requireUnconsumed = false)
            var current = value

            fun update(x: Float) {
              val next = coerceValue(x)
              if (next == current) return
              current = next

              haptic.performHapticFeedback(HapticFeedbackType.SegmentTick)
              onValueChange(next)
            }

            update(down.position.x)

            drag(down.id) { change ->
              update(change.position.x)
              change.consume()
            }
          }
        }
    )

    Box(
      modifier =
        Modifier.graphicsLayer { translationX = thumbOffset.toPx() }
          .size(thumbSize)
          .dropShadow(AppShapes.circle) {
            color = colors.shadowAmbient
            radius = 4f
          }
          .dropShadow(AppShapes.circle) {
            color = colors.shadow
            radius = 8f
            offset = Offset(0f, 1f)
          }
          .border(1.dp, AppTheme.colors.borderDefault, AppShapes.circle)
          .background(AppTheme.colors.surfaceRaised, AppShapes.circle)
    )
  }
}
