package co.typie.screen.editor.editor

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertCountEquals
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasSetTextAction
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.ui.theme.LocalThemeMode
import co.typie.ui.theme.ResolvedThemeMode
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class EditorLoadingInputDesktopTest {
  @Test
  fun loadingSkeletonBlocksEditorInput() = runComposeUiTest {
    var underlyingEvents by mutableIntStateOf(0)

    setContent {
      Box(Modifier.size(width = 320.dp, height = 640.dp).testTag(RootTag)) {
        Box(
          Modifier.fillMaxSize().pointerInput(Unit) {
            awaitPointerEventScope {
              while (true) {
                val event = awaitPointerEvent(PointerEventPass.Main)
                if (event.type == PointerEventType.Press || event.type == PointerEventType.Scroll) {
                  underlyingEvents += 1
                }
              }
            }
          }
        )
        CompositionLocalProvider(
          LocalDensity provides Density(1f),
          LocalThemeMode provides ResolvedThemeMode.Light,
        ) {
          EditorLoadingSkeleton(
            layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 600f),
            topInset = 0.dp,
            background = Color.White,
            modifier = Modifier.fillMaxSize(),
          )
        }
      }
    }
    waitForIdle()

    onAllNodes(hasSetTextAction(), useUnmergedTree = true).assertCountEquals(0)
    onNodeWithTag(RootTag).performTouchInput { click(center) }
    onNodeWithTag(RootTag).performMouseInput { scroll(Offset(x = 0f, y = 120f)) }
    waitForIdle()

    assertEquals(0, underlyingEvents)
  }

  private companion object {
    const val RootTag = "editor-loading-input-root"
  }
}
