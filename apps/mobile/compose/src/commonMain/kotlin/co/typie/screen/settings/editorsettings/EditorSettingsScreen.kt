package co.typie.screen.settings.editorsettings

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import co.typie.domain.settings.SettingControlRow
import co.typie.domain.settings.SettingSwitch
import co.typie.ext.verticalScroll
import co.typie.storage.Preference
import co.typie.ui.component.CardDivider
import co.typie.ui.component.CardSurface
import co.typie.ui.component.Screen
import co.typie.ui.component.SectionTitle
import co.typie.ui.component.Slider
import co.typie.ui.component.Text
import co.typie.ui.component.topbar.ProvideTopBar
import co.typie.ui.component.topbar.topBarScrollOffset
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
fun EditorSettingsScreen() {
  val scrollState = rememberScrollState()
  val haptic = LocalHapticFeedback.current

  ProvideTopBar(
    center = { Text("에디터", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen { contentPadding ->
    Column(
      modifier =
        Modifier.fillMaxSize()
          .verticalScroll(scrollState)
          .padding(contentPadding)
          .padding(AppTheme.spacings.scrollBottomPadding),
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
                color = AppTheme.colors.textMuted,
              )
            }

            Row(
              verticalAlignment = Alignment.CenterVertically,
              horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
              Text("화면 상단", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)

              Slider(
                value = (Preference.typewriterPosition * 100).toFloat().coerceIn(0f, 100f),
                range = 0f..100f,
                step = 5f,
                onDragStart = {},
                onDrag = { next ->
                  haptic.performHapticFeedback(HapticFeedbackType.SegmentTick)
                  Preference.typewriterPosition = next.toDouble() / 100.0
                },
                onDragEnd = {},
                fillColor = AppTheme.colors.textDefault.copy(alpha = 0.75f),
                modifier = Modifier.weight(1f).height(32.dp),
              )

              Text("화면 하단", style = AppTheme.typography.caption, color = AppTheme.colors.textMuted)
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
