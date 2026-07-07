package co.typie.screen.editor.editor.toolbar.bottom

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.click
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.LightAppShadows
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import co.typie.ui.theme.LocalAppShadows
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test

@OptIn(ExperimentalTestApi::class)
class TableSizeSelectorDesktopTest {
  @Test
  fun tappingVisiblePaddedCellSelectsThatTableSize() = runComposeUiTest {
    setContent {
      CompositionLocalProvider(
        LocalAppColors provides LightColors,
        LocalAppShadows provides LightAppShadows,
        LocalThemeMode provides ResolvedThemeMode.Light,
      ) {
        Box(Modifier.size(width = 320.dp, height = 280.dp)) {
          BottomToolbarTableSizeSelector(onEditorMessage = {}, onEditorInputRequest = {})
        }
      }
    }

    waitForIdle()
    onNodeWithText("3×3 삽입").fetchSemanticsNode()

    onRoot().performTouchInput { click(Offset(x = 170f, y = 170f)) }
    waitForIdle()

    onNodeWithText("4×4 삽입").fetchSemanticsNode()
  }
}
