package co.typie.screen.editor.editor.subpane

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.test.swipeWithVelocity
import androidx.compose.ui.unit.dp
import co.typie.ext.ScrollGestureLockScope
import co.typie.ext.verticalScroll
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class EditorResizableSheetSurfaceDesktopTest {
  @Test
  fun dismissReportsStartBeforeDismissed() = runComposeUiTest {
    val events = mutableListOf<String>()

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            keyboardOcclusion = 320.dp,
            minKeyboardVisibleHeight = 240.dp,
            onDismissStarted = { events += "start" },
            onDismissed = { events += "dismissed" },
            onGeometryChanged = {},
          ) {
            Box(Modifier.testTag(DismissButtonTag).fillMaxSize().clickable { dismiss() })
          }
        }
      }
    }
    waitForIdle()

    onNodeWithTag(DismissButtonTag).performClick()
    waitForIdle()

    assertEquals(listOf("start", "dismissed"), events)
  }

  @Test
  fun sheetDragHandleDoesNotFlingSheetContentScroll() = runComposeUiTest {
    var scrollState: ScrollState? = null

    setContent {
      ScrollGestureLockScope {
        Box(Modifier.size(width = 400.dp, height = 800.dp)) {
          EditorResizableSheetSurface(
            initialHeight = 360.dp,
            minHeight = 240.dp,
            dismissThreshold = 128.dp,
            maxTopInset = 0.dp,
            keyboardOcclusion = 320.dp,
            minKeyboardVisibleHeight = 240.dp,
            onDismissed = {},
            onGeometryChanged = {},
          ) {
            val state = remember { ScrollState(initial = 1000) }
            scrollState = state

            Column(Modifier.fillMaxSize()) {
              Box(
                Modifier.testTag(SheetDragHandleTag).fillMaxWidth().height(72.dp).sheetDragHandle()
              )
              Column(Modifier.fillMaxSize().verticalScroll(state)) {
                repeat(80) { Box(Modifier.fillMaxWidth().height(48.dp)) }
              }
            }
          }
        }
      }
    }
    waitForIdle()

    val beforeDrag = checkNotNull(scrollState).value

    onNodeWithTag(SheetDragHandleTag).performTouchInput {
      swipeWithVelocity(start = center, end = center + Offset(x = 0f, y = 160f), endVelocity = 900f)
    }
    waitForIdle()

    assertEquals(beforeDrag, checkNotNull(scrollState).value)
  }

  private companion object {
    const val DismissButtonTag = "dismiss-button"
    const val SheetDragHandleTag = "sheet-drag-handle"
  }
}
