package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.gestures.rememberScrollableState
import androidx.compose.foundation.gestures.scrollable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
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
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.EditorInteractionScope
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.EditorViewportState
import co.typie.navigation.LocalNavigationPopNestedScrollConnection
import co.typie.screen.editor.editor.state.EditorScreenState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorScreenLayoutDesktopTest {
  @Test
  fun toolbarOverlaysWithoutShrinkingViewport() = runComposeUiTest {
    var measuredViewportSize = Size.Zero

    setContent {
      val coroutineScope = rememberCoroutineScope()
      val interactionScope = remember { EditorInteractionScope(coroutineScope = coroutineScope) }
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
        LocalEditorBringIntoViewRequests provides rememberEditorBringIntoViewRequests(),
        LocalEditorInteractionScope provides interactionScope,
        LocalEditorUiState provides remember { EditorUiState() },
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
          body = { interactionModifier -> Box(interactionModifier.fillMaxWidth().height(800.dp)) },
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
      val coroutineScope = rememberCoroutineScope()
      val interactionScope = remember { EditorInteractionScope(coroutineScope = coroutineScope) }
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
        LocalEditorBringIntoViewRequests provides rememberEditorBringIntoViewRequests(),
        LocalEditorInteractionScope provides interactionScope,
        LocalEditorUiState provides remember { EditorUiState() },
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
          body = { interactionModifier -> Box(interactionModifier.fillMaxWidth().height(800.dp)) },
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

  @Test
  fun subPaneNestedScrollDoesNotEnterNavigationPopConnection() = runComposeUiTest {
    var navigationPreScrollCount = 0
    val navigationConnection =
      object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
          navigationPreScrollCount += 1
          return Offset.Zero
        }
      }

    setContent {
      val coroutineScope = rememberCoroutineScope()
      val interactionScope = remember { EditorInteractionScope(coroutineScope = coroutineScope) }
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
      val subPaneScrollState = rememberScrollableState { delta -> delta }

      CompositionLocalProvider(
        LocalEditorBringIntoViewRequests provides rememberEditorBringIntoViewRequests(),
        LocalEditorInteractionScope provides interactionScope,
        LocalEditorUiState provides remember { EditorUiState() },
        LocalNavigationPopNestedScrollConnection provides navigationConnection,
      ) {
        EditorScreenLayout(
          state = remember { EditorScreenState(EditorViewportState()) },
          scrollFrame = scrollFrame,
          visibleArea = visibleArea,
          viewportScrollableState = rememberScrollable2DState { Offset.Zero },
          viewportContentWidth = 320f,
          viewportScrollReconcileMode = EditorViewportScrollReconcileMode.Disabled,
          onMeasuredViewportSizeChange = {},
          header = {},
          body = { interactionModifier -> Box(interactionModifier.fillMaxWidth().height(800.dp)) },
          toolbar = {},
          subPane = {
            Box(
              Modifier.fillMaxSize()
                .testTag(SubPaneTag)
                .scrollable(state = subPaneScrollState, orientation = Orientation.Vertical)
            )
          },
          modifier = Modifier.size(width = 320.dp, height = 640.dp),
        )
      }
    }
    waitForIdle()

    onNodeWithTag(SubPaneTag).performTouchInput {
      swipe(start = center, end = Offset(x = center.x, y = center.y - 120f))
    }
    waitForIdle()

    assertEquals(0, navigationPreScrollCount)
  }

  @Test
  fun siblingSubPanePointerDoesNotEnterViewportGestures() = runComposeUiTest {
    var consumed = Offset.Zero
    lateinit var interactionScope: EditorInteractionScope

    setContent {
      val coroutineScope = rememberCoroutineScope()
      interactionScope = remember { EditorInteractionScope(coroutineScope = coroutineScope) }
      val visibleArea = EditorVisibleArea(viewport = Size(width = 320f, height = 640f))
      val uiState = remember {
        EditorUiState().apply {
          updateExtensionAreaBounds(
            boundsInRoot = Rect(left = 0f, top = 0f, right = 320f, bottom = 640f),
            density = 1f,
          )
        }
      }
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
      val subPaneScrollState = rememberScrollableState { delta -> delta }

      CompositionLocalProvider(
        LocalEditorBringIntoViewRequests provides rememberEditorBringIntoViewRequests(),
        LocalEditorInteractionScope provides interactionScope,
        LocalEditorUiState provides uiState,
      ) {
        EditorScreenLayout(
          state = remember { EditorScreenState(EditorViewportState()) },
          scrollFrame = scrollFrame,
          visibleArea = visibleArea,
          viewportScrollableState =
            rememberScrollable2DState { delta ->
              consumed += delta
              delta
            },
          viewportContentWidth = 320f,
          viewportScrollReconcileMode = EditorViewportScrollReconcileMode.Disabled,
          onMeasuredViewportSizeChange = {},
          header = {},
          body = { interactionModifier -> Box(interactionModifier.fillMaxWidth().height(800.dp)) },
          toolbar = {},
          subPane = {
            Box(
              Modifier.align(Alignment.BottomCenter)
                .fillMaxWidth()
                .height(160.dp)
                .testTag(SubPaneTag)
                .scrollable(state = subPaneScrollState, orientation = Orientation.Vertical)
            )
          },
          modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(LayoutTag),
        )
      }
    }
    waitForIdle()
    onNodeWithTag(LayoutTag).performTouchInput {
      swipe(start = Offset(x = center.x, y = 600f), end = Offset(x = center.x, y = 520f))
    }
    waitForIdle()
    assertEquals(Offset.Zero, consumed)

    onNodeWithTag(LayoutTag).performTouchInput {
      down(pointerId = 0, position = Offset(x = center.x, y = 560f))
      down(pointerId = 1, position = Offset(x = center.x, y = 360f))
    }
    assertEquals(EditorInteractionMode.Idle, interactionScope.controller.interactionMode)
    onNodeWithTag(LayoutTag).performTouchInput {
      up(pointerId = 1)
      up(pointerId = 0)
    }

    onNodeWithTag(LayoutTag).performTouchInput {
      swipe(start = Offset(x = center.x, y = 360f), end = Offset(x = center.x, y = 280f))
    }
    waitForIdle()
    assertTrue(consumed != Offset.Zero)
  }

  @Test
  fun viewportPanStillEntersNavigationPopConnection() = runComposeUiTest {
    var navigationPreScrollCount = 0
    val navigationConnection =
      object : NestedScrollConnection {
        override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
          navigationPreScrollCount += 1
          return Offset.Zero
        }
      }

    setContent {
      val coroutineScope = rememberCoroutineScope()
      val interactionScope = remember { EditorInteractionScope(coroutineScope = coroutineScope) }
      val visibleArea = EditorVisibleArea(viewport = Size(width = 320f, height = 640f))
      val uiState = remember {
        EditorUiState().apply {
          updateExtensionAreaBounds(
            boundsInRoot = Rect(left = 0f, top = 0f, right = 320f, bottom = 640f),
            density = 1f,
          )
        }
      }
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
        LocalEditorBringIntoViewRequests provides rememberEditorBringIntoViewRequests(),
        LocalEditorInteractionScope provides interactionScope,
        LocalEditorUiState provides uiState,
        LocalNavigationPopNestedScrollConnection provides navigationConnection,
      ) {
        EditorScreenLayout(
          state = remember { EditorScreenState(EditorViewportState()) },
          scrollFrame = scrollFrame,
          visibleArea = visibleArea,
          viewportScrollableState = rememberScrollable2DState { delta -> delta },
          viewportContentWidth = 320f,
          viewportScrollReconcileMode = EditorViewportScrollReconcileMode.Disabled,
          onMeasuredViewportSizeChange = {},
          header = {},
          body = { interactionModifier -> Box(interactionModifier.fillMaxWidth().height(800.dp)) },
          toolbar = {},
          modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(LayoutTag),
        )
      }
    }
    waitForIdle()

    onNodeWithTag(LayoutTag).performTouchInput {
      swipe(start = center, end = Offset(x = center.x, y = center.y - 120f))
    }
    waitForIdle()

    assertTrue(navigationPreScrollCount > 0)
  }

  private companion object {
    const val LayoutTag = "editor-screen-layout"
    const val SubPaneTag = "editor-screen-layout-subpane"
  }
}
