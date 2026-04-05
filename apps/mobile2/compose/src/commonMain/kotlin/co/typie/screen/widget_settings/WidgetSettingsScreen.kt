package co.typie.screen.widget_settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.ext.navigationBarsPadding
import co.typie.ext.verticalScroll
import co.typie.service.EditorPreferencesService
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
import org.koin.compose.koinInject

@Composable
fun WidgetSettingsScreen() {
  val editorPreferences = koinInject<EditorPreferencesService>()
  val scrollState = rememberScrollState()

  val characterCountFloatingEnabled = editorPreferences.characterCountFloatingEnabled
  val widgetAutoFadeEnabled = editorPreferences.widgetAutoFadeEnabled

  ProvideTopBar(
    leading = { TopBarBackButton() },
    center = { Text("위젯", style = AppTheme.typography.title) },
    scrollOffset = scrollState.topBarScrollOffset(),
  )

  Screen(
    scrollState = scrollState,
    background = AppTheme.colors.surfaceBase,
    verticalArrangement = Arrangement.spacedBy(16.dp),
  ) {
      Text(
        "위젯",
        style = AppTheme.typography.display,
        modifier = Modifier.padding(top = 4.dp),
      )

      WidgetSettingsSection(title = "위젯 설정") {
        SettingControlRow(
          label = "글자 수 위젯",
          description = "에디터에서 글자 수를 표시합니다.",
          onClick = {
            editorPreferences.characterCountFloatingEnabled = !characterCountFloatingEnabled
          },
          trailing = {
            SettingSwitch(
              checked = characterCountFloatingEnabled,
              onCheckedChange = { next ->
                editorPreferences.characterCountFloatingEnabled = next
              },
            )
          },
        )

        if (characterCountFloatingEnabled) {
          CardDivider(inset = 20.dp)
          SettingControlRow(
            label = "위젯 자동 페이드 인/아웃",
            description = "타이핑, 스크롤 시 위젯이 잠시 사라집니다.",
            onClick = {
              editorPreferences.widgetAutoFadeEnabled = !widgetAutoFadeEnabled
            },
            trailing = {
              SettingSwitch(
                checked = widgetAutoFadeEnabled,
                onCheckedChange = { next ->
                  editorPreferences.widgetAutoFadeEnabled = next
                },
              )
            },
          )
        }
      }

      Spacer(Modifier.height(72.dp))
  }
}

@Composable
private fun WidgetSettingsSection(
  title: String,
  content: @Composable ColumnScope.() -> Unit,
) {
  Column(
    modifier = Modifier.fillMaxWidth(),
    verticalArrangement = Arrangement.spacedBy(12.dp),
  ) {
    SectionTitle(
      title,
      modifier = Modifier.padding(top = 4.dp),
    )

    CardSurface(
      modifier = Modifier.fillMaxWidth(),
    ) {
      Column(content = content)
    }
  }
}
