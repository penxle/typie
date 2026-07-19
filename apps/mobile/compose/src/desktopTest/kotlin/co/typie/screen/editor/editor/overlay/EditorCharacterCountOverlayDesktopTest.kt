package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.screen.editor.editor.layout.viewportDirectControl
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class EditorCharacterCountOverlayDesktopTest {
  @Test
  fun tapDetectorAcceptsDownConsumedByDirectControlMarker() = runComposeUiTest {
    var tapCount = 0
    setContent {
      Box(
        Modifier.size(120.dp).testTag(OverlayTag).viewportDirectControl().pointerInput(Unit) {
          detectTapAfterConsumedDown { tapCount += 1 }
        }
      )
    }

    onNodeWithTag(OverlayTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    assertEquals(1, tapCount)
  }

  @Test
  fun consumedDragCancelsTap() = runComposeUiTest {
    var dragCount = 0
    var tapCount = 0
    setContent {
      Box(
        Modifier.size(120.dp)
          .testTag(OverlayTag)
          .viewportDirectControl()
          .pointerInput(Unit) {
            detectDragGestures { change, _ ->
              change.consume()
              dragCount += 1
            }
          }
          .pointerInput(Unit) { detectTapAfterConsumedDown { tapCount += 1 } }
      )
    }

    onNodeWithTag(OverlayTag).performTouchInput {
      down(center)
      moveBy(Offset(x = 40f, y = 0f))
      up()
    }
    waitForIdle()

    assertEquals(1, dragCount)
    assertEquals(0, tapCount)
  }

  private companion object {
    const val OverlayTag = "editor-character-count-overlay"
  }
}
