package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.rememberScrollable2DState
import androidx.compose.foundation.gestures.rememberScrollableState
import androidx.compose.foundation.gestures.scrollable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.pointer.pointerInput
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
import co.typie.editor.ffi.Size as PageSize
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
import co.typie.navigation.LocalNavigationPopNestedScroll
import co.typie.navigation.NavigationPopNestedScroll
import co.typie.screen.editor.editor.overlay.EditorScrollbars
import co.typie.screen.editor.editor.state.EditorScreenState
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorScreenLayoutDesktopTest {
  @Test
  fun viewportOverlayWheelReachesViewport() = runComposeUiTest {
    val fixture = ViewportOverlayFixture()
    setViewportOverlayContent(fixture = fixture, viewportOverlay = { ViewportOverlayTarget() })

    onNodeWithTag(LayoutTag).performMouseInput {
      moveTo(Offset(x = 30f, y = 16f))
      scroll(Offset(x = 0f, y = 120f))
    }
    waitForIdle()

    assertTrue(fixture.scrollDeltas.isNotEmpty())
  }

  @Test
  fun viewportOverlayConsumedDownDoesNotStartEditorGesture() = runComposeUiTest {
    val fixture = ViewportOverlayFixture()
    setViewportOverlayContent(fixture = fixture, viewportOverlay = { ViewportOverlayTarget() })

    onNodeWithTag(ViewportOverlayTag).performTouchInput {
      down(center)
      up()
    }
    waitForIdle()

    assertEquals(0, fixture.editorPointerInputCount)
    assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
  }

  @Test
  fun scrollbarOwnsDownAndSharesWheelWithViewport() = runComposeUiTest {
    val fixture = ViewportOverlayFixture()
    setViewportOverlayContent(
      fixture = fixture,
      viewportOverlay = {
        CompositionLocalProvider(LocalAppColors provides LightColors) {
          EditorScrollbars(
            viewportState = fixture.viewportState,
            visibleArea = fixture.visibleArea,
            layoutSpec = fixture.layoutSpec,
            pageSizes = fixture.pageSizes,
            displayZoom = 1f,
          )
        }
      },
    )
    runOnIdle { fixture.viewportState.scrollToY(1f) }
    waitForIdle()

    onNodeWithTag(LayoutTag).performTouchInput {
      down(Offset(x = TestViewportSize.width - 20f, y = 32f))
      up()
    }
    waitForIdle()
    assertEquals(0, fixture.editorPointerInputCount, "scrollbar down reached editor")

    onNodeWithTag(LayoutTag).performMouseInput {
      moveTo(center)
      scroll(Offset(x = 0f, y = 120f))
    }
    waitForIdle()
    assertTrue(fixture.scrollDeltas.isNotEmpty(), "wheel outside the thumb did not reach viewport")
    runOnIdle { fixture.scrollDeltas.clear() }

    onNodeWithTag(LayoutTag).performMouseInput {
      moveTo(Offset(x = TestViewportSize.width - 20f, y = 32f))
      scroll(Offset(x = 0f, y = 120f))
    }
    waitForIdle()

    assertTrue(fixture.scrollDeltas.isNotEmpty(), "wheel over the thumb did not reach viewport")
  }

  @Test
  fun regularOverlayStillBlocksViewportWheel() = runComposeUiTest {
    val fixture = ViewportOverlayFixture()
    setViewportOverlayContent(fixture = fixture, overlay = { ViewportOverlayTarget() })

    onNodeWithTag(LayoutTag).performMouseInput {
      moveTo(center)
      scroll(Offset(x = 0f, y = 120f))
    }
    waitForIdle()

    assertTrue(fixture.scrollDeltas.isEmpty())
  }

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
  fun disabledEditorInteractionDoesNotPanFromTouchOrWheel() = runComposeUiTest {
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
          editorInteractionEnabled = false,
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
    var navigationDragCount = 0
    val navigationPopNestedScroll =
      NavigationPopNestedScroll().apply {
        update(
          activationDistance = 15f,
          canStart = { true },
          onStart = {},
          onDrag = { navigationDragCount += 1 },
          onRelease = {},
          onCancel = {},
        )
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
        LocalNavigationPopNestedScroll provides navigationPopNestedScroll,
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
      swipe(start = center, end = Offset(x = center.x + 120f, y = center.y))
    }
    waitForIdle()

    assertEquals(0, navigationDragCount)
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
          updateInteractionSurfaceBounds(
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
    var navigationDragCount = 0
    val navigationPopNestedScroll =
      NavigationPopNestedScroll().apply {
        update(
          activationDistance = 15f,
          canStart = { true },
          onStart = {},
          onDrag = { navigationDragCount += 1 },
          onRelease = {},
          onCancel = {},
        )
      }

    setContent {
      val coroutineScope = rememberCoroutineScope()
      val interactionScope = remember { EditorInteractionScope(coroutineScope = coroutineScope) }
      val visibleArea = EditorVisibleArea(viewport = Size(width = 320f, height = 640f))
      val uiState = remember {
        EditorUiState().apply {
          updateInteractionSurfaceBounds(
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
        LocalNavigationPopNestedScroll provides navigationPopNestedScroll,
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
          modifier = Modifier.size(width = 320.dp, height = 640.dp).testTag(LayoutTag),
        )
      }
    }
    waitForIdle()

    // NavigationStack supplies this root pointer membership in production. This isolated layout
    // test provides the same admission state before exercising the nested-scroll connection.
    navigationPopNestedScroll.updatePressedDragPointerCount(
      count = 1,
      downInSystemBackZone = false,
      pointerId = 1L,
      position = Offset.Zero,
    )
    navigationPopNestedScroll.updatePressedDragPointerCount(
      count = 1,
      downInSystemBackZone = false,
      pointerId = 1L,
      position = Offset(x = 120f, y = 0f),
    )
    onNodeWithTag(LayoutTag).performTouchInput {
      swipe(start = center, end = Offset(x = center.x + 120f, y = center.y))
    }
    navigationPopNestedScroll.updatePressedDragPointerCount(count = 0, downInSystemBackZone = false)
    waitForIdle()

    assertTrue(navigationDragCount > 0)
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setViewportOverlayContent(
    fixture: ViewportOverlayFixture,
    viewportOverlay: @Composable BoxScope.() -> Unit = {},
    overlay: @Composable () -> Unit = {},
  ) {
    setContent {
      val coroutineScope = rememberCoroutineScope()
      val interactionScope = remember { EditorInteractionScope(coroutineScope = coroutineScope) }
      fixture.interactionScope = interactionScope
      CompositionLocalProvider(
        LocalEditorBringIntoViewRequests provides rememberEditorBringIntoViewRequests(),
        LocalEditorInteractionScope provides interactionScope,
        LocalEditorUiState provides fixture.uiState,
      ) {
        EditorScreenLayout(
          state = remember { EditorScreenState(fixture.viewportState) },
          scrollFrame = fixture.scrollFrame,
          visibleArea = fixture.visibleArea,
          viewportScrollableState =
            rememberScrollable2DState { delta ->
              fixture.scrollDeltas += delta
              delta
            },
          viewportContentWidth = TestViewportSize.width,
          viewportScrollReconcileMode = EditorViewportScrollReconcileMode.Disabled,
          onEditorPointerInput = { fixture.editorPointerInputCount += 1 },
          onMeasuredViewportSizeChange = {},
          header = {},
          body = { interactionModifier -> Box(interactionModifier.fillMaxWidth().height(800.dp)) },
          viewportOverlay = viewportOverlay,
          overlay = overlay,
          toolbar = {},
          modifier =
            Modifier.size(width = TestViewportSize.width.dp, height = TestViewportSize.height.dp)
              .testTag(LayoutTag),
        )
      }
    }
    waitForIdle()
  }

  @Composable
  private fun ViewportOverlayTarget() {
    Box(
      Modifier.fillMaxSize()
        .testTag(ViewportOverlayTag)
        .pointerInput(Unit) { detectDragGestures { change, _ -> change.consume() } }
        .pointerInput(Unit) { detectTapGestures(onTap = {}) }
    )
  }

  private class ViewportOverlayFixture {
    val viewportState = EditorViewportState()
    val visibleArea = EditorVisibleArea(viewport = TestViewportSize)
    val uiState = EditorUiState()
    lateinit var interactionScope: EditorInteractionScope
    val scrollDeltas = mutableListOf<Offset>()
    var editorPointerInputCount = 0

    val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = TestViewportSize.width,
        pageHeight = TestViewportSize.height,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      )
    val pageSizes =
      listOf(PageSize(width = TestViewportSize.width, height = TestViewportSize.height * 2f))

    val scrollFrame =
      EditorScrollFrame(
        state = EditorState.Initial,
        layoutSpec = layoutSpec,
        displayZoom = 1f,
        visibleArea = visibleArea,
        autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea),
        headerHeight = 0f,
        density = 1f,
        editorBounds = EditorBoundsInContainer(),
      )
  }

  private companion object {
    const val LayoutTag = "editor-screen-layout"
    const val SubPaneTag = "editor-screen-layout-subpane"
    const val ViewportOverlayTag = "editor-screen-layout-viewport-overlay-target"
    val TestViewportSize = Size(width = 320f, height = 640f)
  }
}
