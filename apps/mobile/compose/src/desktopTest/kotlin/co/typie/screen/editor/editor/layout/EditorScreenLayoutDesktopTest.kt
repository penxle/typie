package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.test.swipe
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorState
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.EditorViewportState
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class EditorScreenLayoutDesktopTest {
  @Test
  fun toolbarOverlaysWithoutShrinkingViewport() = runComposeUiTest {
    var measuredViewportSize = Size.Zero

    setContent {
      val visibleArea = EditorVisibleArea(viewport = Size(width = 320f, height = 640f))
      val scrollFrame =
        EditorScrollFrame(
          state = EditorState.Initial,
          layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 320f),
          displayZoom = 1f,
          visibleArea = visibleArea,
          autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea),
          headerHeight = 0f,
          density = 1f,
          editorBounds = EditorBoundsInContainer(),
        )

      CompositionLocalProvider(
        LocalEditorBringIntoViewRequests provides rememberEditorBringIntoViewRequests()
      ) {
        EditorScreenLayout(
          state = remember { EditorScreenState(EditorViewportState()) },
          scrollFrame = scrollFrame,
          visibleArea = visibleArea,
          viewportScrollableState = rememberScrollable2DState { Offset.Zero },
          viewportContentWidth = 320f,
          viewportScrollReconcileMode = EditorViewportScrollReconcileMode.Disabled,
          onMeasuredViewportSizeChange = { measuredViewportSize = it },
          header = {},
          body = { Box(Modifier.fillMaxWidth().height(800.dp)) },
          toolbar = { Box(Modifier.fillMaxWidth().height(96.dp)) },
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        )
      }
    }

    waitForIdle()

    assertEquals(Size(width = 320f, height = 640f), measuredViewportSize)
  }

  @Test
  fun disabledViewportInputDoesNotPanFromTouchOrWheel() = runComposeUiTest {
    var consumed = Offset.Zero

    setContent {
      val visibleArea = EditorVisibleArea(viewport = Size(width = 320f, height = 640f))
      val scrollFrame =
        EditorScrollFrame(
          state = EditorState.Initial,
          layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 320f),
          displayZoom = 1f,
          visibleArea = visibleArea,
          autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea),
          headerHeight = 0f,
          density = 1f,
          editorBounds = EditorBoundsInContainer(),
        )
      CompositionLocalProvider(
        LocalEditorBringIntoViewRequests provides rememberEditorBringIntoViewRequests()
      ) {
        EditorScreenLayout(
          state = remember { EditorScreenState(EditorViewportState()) },
          scrollFrame = scrollFrame,
          visibleArea = visibleArea,
          viewportScrollableState =
            rememberScrollable2DState {
              consumed += it
              it
            },
          viewportContentWidth = 320f,
          viewportInputEnabled = false,
          viewportScrollReconcileMode = EditorViewportScrollReconcileMode.Disabled,
          onMeasuredViewportSizeChange = {},
          header = {},
          body = { Box(Modifier.fillMaxWidth().height(800.dp)) },
          toolbar = {},
          modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(LayoutTag),
        )
      }
    }
    waitForIdle()

    onNodeWithTag(LayoutTag).performTouchInput {
      swipe(start = center, end = Offset(x = center.x, y = center.y - 120f))
    }
    onNodeWithTag(LayoutTag).performMouseInput { scroll(Offset(x = 0f, y = 120f)) }
    waitForIdle()

    assertEquals(Offset.Zero, consumed)
  }

  private companion object {
    const val LayoutTag = "editor-screen-layout"
  }
}
