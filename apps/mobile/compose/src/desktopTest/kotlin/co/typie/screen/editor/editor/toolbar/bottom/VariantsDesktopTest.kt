package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.HorizontalRuleVariant
import co.typie.editor.ffi.TableBorderStyle
import co.typie.screen.editor.editor.toolbar.HorizontalRuleVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.TableBorderStylePanelTarget
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test

@OptIn(ExperimentalTestApi::class)
class VariantsDesktopTest {
  @Test
  fun horizontal_rule_panel_reveals_current_variant() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(width = 320.dp, height = 180.dp)) {
          BottomToolbarHorizontalRuleVariants(
            target =
              HorizontalRuleVariantPanelTarget.Existing(
                nodeId = "horizontal-rule",
                currentVariant = HorizontalRuleVariant.Zigzag,
              ),
            onEditorMessage = {},
            onEditorInputRequest = {},
          )
        }
      }
    }

    waitForIdle()

    onNodeWithText("지그재그").assertIsDisplayed()
  }

  @Test
  fun table_border_styles_panel_composes_all_variants() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(width = 320.dp, height = 260.dp)) {
          BottomToolbarTableBorderStyles(
            target =
              TableBorderStylePanelTarget(tableId = "table", currentStyle = TableBorderStyle.Solid),
            onEditorMessage = {},
            onEditorInputRequest = {},
          )
        }
      }
    }

    waitForIdle()

    onNodeWithText("실선").fetchSemanticsNode()
    onNodeWithText("파선").fetchSemanticsNode()
    onNodeWithText("점선").fetchSemanticsNode()
    onNodeWithText("없음").fetchSemanticsNode()
  }
}
