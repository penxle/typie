package co.typie.screen.editor.editor.layout

import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.pointer.PointerButtons
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerId
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.scene.ComposeScenePointer
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertIsFocused
import androidx.compose.ui.test.assertIsNotFocused
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.pan
import androidx.compose.ui.test.performKeyInput
import androidx.compose.ui.test.performMouseInput
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.performTrackpadInput
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.test.scale
import androidx.compose.ui.test.swipe
import androidx.compose.ui.test.v2.runSkikoComposeUiTest
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.EditorZoomController
import co.typie.editor.FakeFfiEditor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.interaction.EditorInteractionMode
import co.typie.editor.interaction.EditorInteractionScope
import co.typie.editor.interaction.LocalEditorInteractionScope
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.runtime.EditorBoundsInContainer
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.runtime.LocalEditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorScrollFrame
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.rememberEditorBringIntoViewRequests
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.viewport.EditorViewportState
import co.typie.ext.ScrollGestureLockState
import co.typie.navigation.LocalNavigationPopNestedScroll
import co.typie.navigation.NavigationPopNestedScroll
import co.typie.screen.editor.editor.overlay.EditorScrollbars
import co.typie.screen.editor.editor.state.EditorScreenState
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

@OptIn(ExperimentalTestApi::class)
class EditorScreenLayoutDesktopTest {
  @Test
  fun headerWheelReachesViewportScrollableState() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(fixture)

      onNodeWithTag(LayoutTag).performMouseInput {
        moveTo(Offset(x = 280f, y = HeaderHeightPx / 2f))
        scroll(Offset(x = 0f, y = 120f))
      }
      waitForIdle()

      assertTrue(fixture.scrollDeltas.isNotEmpty())
    } finally {
      fixture.close()
    }
  }

  @Test
  fun stationaryHeaderFieldTapRetainsFocusAndPlacesCaret() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(fixture)

      onNodeWithTag(HeaderFieldTag).performTouchInput {
        down(Offset(x = 180f, y = center.y))
        up()
      }
      waitForIdle()

      onNodeWithTag(HeaderFieldTag).assertIsFocused()
      assertTrue(fixture.headerFocused)
      assertNotEquals(TextRange.Zero, fixture.fieldValue.selection)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun headerFieldPanClaimsViewportWithoutPromotingReleaseToFieldOrEditor() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(fixture)

      onNodeWithTag(LayoutTag).performTouchInput {
        down(pointerId = 0, position = Offset(x = 80f, y = 40f))
        moveTo(pointerId = 0, position = Offset(x = 82f, y = HeaderHeightPx + 100f))
        up(pointerId = 0)
      }
      waitForIdle()

      assertTrue(fixture.scrollDeltas.isNotEmpty())
      onNodeWithTag(HeaderFieldTag).assertIsNotFocused()
      assertEquals(TextRange.Zero, fixture.fieldValue.selection)
      assertTrue(fixture.fake.enqueued.isEmpty())
      assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun viewportDirectControlConsumesBeforeHeaderOwnerAdmission() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    var controlDownCount = 0
    try {
      setHeaderInputContent(
        fixture = fixture,
        viewportOverlay = {
          Box(
            Modifier.width(160.dp)
              .height(HeaderHeightPx.dp)
              .testTag(HeaderControlTag)
              .viewportDirectControl()
              .pointerInput(Unit) {
                awaitEachGesture {
                  awaitFirstDown(requireUnconsumed = false)
                  controlDownCount += 1
                }
              }
          )
        },
      )

      onNodeWithTag(HeaderControlTag).performTouchInput {
        down(Offset(x = 80f, y = 40f))
        moveTo(Offset(x = 82f, y = HeaderHeightPx + 100f))
        up()
      }
      waitForIdle()

      assertEquals(1, controlDownCount)
      assertTrue(fixture.scrollDeltas.isEmpty())
      assertEquals(0, fixture.editorPointerInputCount)
      assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun viewportDirectControlMarksOnlyItsOwnHitRegion() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(
        fixture = fixture,
        viewportOverlay = {
          Box(
            Modifier.width(160.dp)
              .height(HeaderHeightPx.dp)
              .testTag(HeaderControlTag)
              .viewportDirectControl()
          )
        },
      )

      onNodeWithTag(LayoutTag).performTouchInput {
        down(pointerId = 0, position = Offset(x = 80f, y = 40f))
        moveTo(pointerId = 0, position = Offset(x = 82f, y = HeaderHeightPx + 100f))
        up(pointerId = 0)
      }
      waitForIdle()
      assertTrue(fixture.scrollDeltas.isEmpty())
      assertEquals(0, fixture.editorPointerInputCount)

      runOnIdle {
        fixture.scrollDeltas.clear()
        fixture.editorPointerInputCount = 0
      }
      onNodeWithTag(LayoutTag).performTouchInput {
        down(pointerId = 0, position = Offset(x = 280f, y = 40f))
        moveTo(pointerId = 0, position = Offset(x = 282f, y = HeaderHeightPx + 100f))
        up(pointerId = 0)
      }
      waitForIdle()

      assertTrue(fixture.scrollDeltas.isNotEmpty())
      assertTrue(fixture.editorPointerInputCount > 0)
      assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
    } finally {
      fixture.close()
    }
  }

  @OptIn(InternalComposeUiApi::class)
  @Test
  fun stalePrimaryScrollOverViewportDirectControlReachesViewport() = runSkikoComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(
        fixture = fixture,
        viewportOverlay = {
          Box(Modifier.width(160.dp).height(HeaderHeightPx.dp).viewportDirectControl())
        },
      )

      runOnIdle {
        scene.sendPointerEvent(
          eventType = PointerEventType.Scroll,
          position = Offset(x = 80f, y = 40f),
          scrollDelta = Offset(x = 0f, y = -24f),
          timeMillis = 100L,
          type = PointerType.Mouse,
          buttons = PointerButtons(isPrimaryPressed = true),
        )
      }
      waitForIdle()

      assertTrue(fixture.scrollDeltas.isNotEmpty())
    } finally {
      fixture.close()
    }
  }

  @Test
  fun controlFirstMixedPointerSequenceSuppressesEditorUntilAllUp() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    var controlDragCount = 0
    try {
      setHeaderInputContent(
        fixture = fixture,
        viewportOverlay = { MixedPointerControl { controlDragCount += 1 } },
      )
      val initialZoom = fixture.zoomController.displayZoom
      val root = onNodeWithTag(LayoutTag)

      root.performTouchInput {
        down(pointerId = 0, position = ControlPointerStart)
        down(pointerId = 1, position = EditorPointerStart)
        updatePointerTo(pointerId = 0, position = ControlPointerMove)
        updatePointerTo(pointerId = 1, position = EditorPointerMove)
        move()
      }
      waitForIdle()

      assertTrue(controlDragCount > 0)
      assertMixedEditorSideSuppressed(fixture = fixture, expectedZoom = initialZoom)

      root.performTouchInput {
        up(pointerId = 0)
        updatePointerTo(pointerId = 1, position = EditorSurvivorMove)
        move()
        up(pointerId = 1)
      }
      waitForIdle()

      assertMixedEditorSideSuppressed(fixture = fixture, expectedZoom = initialZoom)
      assertFreshHeaderBodyPinchWorks(fixture)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun editorFirstMixedPointerSequenceSuppressesEditorUntilAllUp() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    var controlDragCount = 0
    try {
      setHeaderInputContent(
        fixture = fixture,
        viewportOverlay = { MixedPointerControl { controlDragCount += 1 } },
      )
      val initialZoom = fixture.zoomController.displayZoom
      val root = onNodeWithTag(LayoutTag)

      root.performTouchInput {
        down(pointerId = 0, position = EditorPointerStart)
        down(pointerId = 1, position = ControlPointerStart)
        updatePointerTo(pointerId = 0, position = EditorPointerMove)
        updatePointerTo(pointerId = 1, position = ControlPointerMove)
        move()
      }
      waitForIdle()

      assertTrue(controlDragCount > 0)
      assertMixedEditorSideSuppressed(fixture = fixture, expectedZoom = initialZoom)

      root.performTouchInput {
        up(pointerId = 1)
        updatePointerTo(pointerId = 0, position = EditorSurvivorMove)
        move()
        up(pointerId = 0)
      }
      waitForIdle()

      assertMixedEditorSideSuppressed(fixture = fixture, expectedZoom = initialZoom)
      assertFreshHeaderBodyPinchWorks(fixture)
    } finally {
      fixture.close()
    }
  }

  @OptIn(InternalComposeUiApi::class)
  @Test
  fun sameEventMixedPointerDownsSuppressEditorUntilAllUp() = runSkikoComposeUiTest {
    val fixture = HeaderInputFixture()
    var controlDragCount = 0
    try {
      setHeaderInputContent(
        fixture = fixture,
        viewportOverlay = { MixedPointerControl { controlDragCount += 1 } },
      )
      val initialZoom = fixture.zoomController.displayZoom

      runOnIdle {
        scene.sendPointerEvent(
          eventType = PointerEventType.Press,
          pointers =
            listOf(
              touchScenePointer(id = 0L, position = ControlPointerStart),
              touchScenePointer(id = 1L, position = EditorPointerStart),
            ),
          timeMillis = 100L,
        )
        scene.sendPointerEvent(
          eventType = PointerEventType.Move,
          pointers =
            listOf(
              touchScenePointer(id = 0L, position = ControlPointerMove),
              touchScenePointer(id = 1L, position = EditorPointerMove),
            ),
          timeMillis = 116L,
        )
      }
      waitForIdle()

      assertTrue(controlDragCount > 0)
      assertMixedEditorSideSuppressed(fixture = fixture, expectedZoom = initialZoom)

      runOnIdle {
        scene.sendPointerEvent(
          eventType = PointerEventType.Release,
          pointers =
            listOf(
              touchScenePointer(id = 0L, position = ControlPointerMove, pressed = false),
              touchScenePointer(id = 1L, position = EditorPointerMove),
            ),
          timeMillis = 132L,
        )
        scene.sendPointerEvent(
          eventType = PointerEventType.Move,
          pointers = listOf(touchScenePointer(id = 1L, position = EditorSurvivorMove)),
          timeMillis = 148L,
        )
        scene.sendPointerEvent(
          eventType = PointerEventType.Release,
          pointers =
            listOf(touchScenePointer(id = 1L, position = EditorSurvivorMove, pressed = false)),
          timeMillis = 164L,
        )
      }
      waitForIdle()

      assertMixedEditorSideSuppressed(fixture = fixture, expectedZoom = initialZoom)
      assertFreshHeaderBodyPinchWorks(fixture)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun scrollSemanticsOwnerStaysFixedWhileHeaderAndBodyShareOnlyVerticalTranslation() =
    runComposeUiTest {
      val fixture = HeaderInputFixture()
      try {
        setHeaderInputContent(fixture)

        val layoutBounds = onNodeWithTag(LayoutTag).fetchSemanticsNode().boundsInRoot
        val ownerBefore =
          onAllNodes(HasScrollByAction, useUnmergedTree = true)
            .fetchSemanticsNodes()
            .single { node -> node.boundsInRoot.width >= TestViewportSize.width }
            .boundsInRoot
        val headerBefore = checkNotNull(fixture.headerPositionInRoot)
        val bodyBefore = checkNotNull(fixture.bodyPositionInRoot)

        assertEquals(layoutBounds, ownerBefore)

        runOnIdle { fixture.viewportState.scrollTo(Offset(x = 40f, y = 60f)) }
        waitForIdle()

        val ownerAfter =
          onAllNodes(HasScrollByAction, useUnmergedTree = true)
            .fetchSemanticsNodes()
            .single { node -> node.boundsInRoot.width >= TestViewportSize.width }
            .boundsInRoot
        val headerAfter = checkNotNull(fixture.headerPositionInRoot)
        val bodyAfter = checkNotNull(fixture.bodyPositionInRoot)

        assertEquals(layoutBounds, ownerAfter)
        assertEquals(headerBefore.y - 60f, headerAfter.y)
        assertEquals(bodyBefore.y - 60f, bodyAfter.y)
        assertEquals(headerBefore.x, headerAfter.x)
        assertEquals(bodyBefore.x - 40f, bodyAfter.x)
      } finally {
        fixture.close()
      }
    }

  @Test
  fun twoHeaderPointersPinchViewport() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(fixture)
      val initialZoom = fixture.zoomController.displayZoom
      val root = onNodeWithTag(LayoutTag)

      root.performTouchInput {
        down(pointerId = 0, position = Offset(x = 80f, y = 30f))
        down(pointerId = 1, position = Offset(x = 160f, y = 50f))
        updatePointerTo(pointerId = 0, position = Offset(x = 60f, y = 25f))
        updatePointerTo(pointerId = 1, position = Offset(x = 180f, y = 55f))
        move()
      }
      assertEquals(
        EditorInteractionMode.ViewportZooming,
        fixture.interactionScope.controller.interactionMode,
      )
      assertTrue(fixture.zoomController.displayZoom > initialZoom)

      root.performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
      waitForIdle()
      assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
      assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun headerAndBodyPointersPinchViewport() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(fixture)
      val initialZoom = fixture.zoomController.displayZoom
      val root = onNodeWithTag(LayoutTag)

      root.performTouchInput {
        down(pointerId = 0, position = Offset(x = 100f, y = 40f))
        down(pointerId = 1, position = Offset(x = 140f, y = HeaderHeightPx + 80f))
        updatePointerTo(pointerId = 0, position = Offset(x = 90f, y = 20f))
        updatePointerTo(pointerId = 1, position = Offset(x = 150f, y = HeaderHeightPx + 110f))
        move()
      }
      assertEquals(
        EditorInteractionMode.ViewportZooming,
        fixture.interactionScope.controller.interactionMode,
      )
      assertTrue(fixture.zoomController.displayZoom > initialZoom)

      root.performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
      waitForIdle()
      assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
      assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun metaModifiedWheelWithHeaderFocalZoomsViewport() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(fixture)
      val initialZoom = fixture.zoomController.displayZoom

      val root = onNodeWithTag(LayoutTag)
      root.performKeyInput { keyDown(Key.MetaLeft) }
      root.performMouseInput {
        moveTo(Offset(x = 280f, y = HeaderHeightPx / 2f))
        scroll(Offset(x = 0f, y = -48f))
      }
      root.performKeyInput { keyUp(Key.MetaLeft) }
      mainClock.advanceTimeBy(100)
      waitForIdle()

      assertTrue(fixture.zoomController.displayZoom > initialZoom)
      assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun commonTrackpadScaleWithHeaderFocalZoomsViewport() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(fixture)
      val initialZoom = fixture.zoomController.displayZoom

      onNodeWithTag(LayoutTag).performTrackpadInput {
        updatePointerTo(Offset(x = 280f, y = HeaderHeightPx / 2f))
        scale(1.2f)
      }
      waitForIdle()

      assertTrue(fixture.zoomController.displayZoom > initialZoom)
      assertEquals(fixture.zoomController.displayZoom, fixture.zoomController.renderZoom)
      assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
    } finally {
      fixture.close()
    }
  }

  @Test
  fun commonTrackpadPanWithHeaderFocalScrollsViewport() = runComposeUiTest {
    val fixture = HeaderInputFixture()
    try {
      setHeaderInputContent(fixture)

      onNodeWithTag(LayoutTag).performTrackpadInput {
        updatePointerTo(Offset(x = 280f, y = HeaderHeightPx / 2f))
        pan(Offset(x = 0f, y = -80f))
      }
      waitForIdle()

      assertTrue(fixture.scrollDeltas.isNotEmpty())
    } finally {
      fixture.close()
    }
  }

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
          body = { Box(Modifier.fillMaxWidth().height(800.dp)) },
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
          body = { Box(Modifier.fillMaxWidth().height(800.dp)) },
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
            boundsInRoot = Rect(left = 0f, top = HeaderHeightPx, right = 320f, bottom = 640f),
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
          headerHeight = HeaderHeightPx,
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
          header = { Box(Modifier.fillMaxWidth().height(HeaderHeightPx.dp).testTag(HeaderTag)) },
          body = { Box(Modifier.fillMaxWidth().height(800.dp).testTag(BodyTag)) },
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
      position = Offset(x = 80f, y = HeaderHeightPx / 2f),
    )
    navigationPopNestedScroll.updatePressedDragPointerCount(
      count = 1,
      downInSystemBackZone = false,
      pointerId = 1L,
      position = Offset(x = 200f, y = HeaderHeightPx / 2f),
    )
    onNodeWithTag(LayoutTag).performTouchInput {
      swipe(
        start = Offset(x = 80f, y = HeaderHeightPx / 2f),
        end = Offset(x = 200f, y = HeaderHeightPx / 2f),
      )
    }
    navigationPopNestedScroll.updatePressedDragPointerCount(count = 0, downInSystemBackZone = false)
    waitForIdle()

    assertTrue(navigationDragCount > 0)
  }

  private fun androidx.compose.ui.test.ComposeUiTest.setHeaderInputContent(
    fixture: HeaderInputFixture,
    viewportOverlay: @Composable BoxScope.() -> Unit = {},
  ) {
    setContent {
      val interactionScope = remember {
        EditorInteractionScope(coroutineScope = fixture.coroutineScope)
      }
      fixture.interactionScope = interactionScope
      val bringIntoViewRequests = remember { EditorBringIntoViewRequests() }
      val scrollGestureLockState = remember { ScrollGestureLockState() }
      val viewportScrollableState = rememberScrollable2DState { delta ->
        fixture.scrollDeltas += delta
        delta
      }

      SideEffect {
        interactionScope.update(
          editor = fixture.editor,
          bringIntoViewRequests = bringIntoViewRequests,
          uiState = fixture.uiState,
          visibleArea = fixture.visibleArea,
          viewportState = fixture.viewportState,
          density = 1f,
          scrollGestureLockState = scrollGestureLockState,
          viewportZoomConfig = fixture.viewportZoomConfig,
          layoutSpec = fixture.layoutSpec,
          onSelectionHaptic = {},
          onRequestSoftwareKeyboard = {},
        )
      }

      CompositionLocalProvider(
        LocalEditorBringIntoViewRequests provides bringIntoViewRequests,
        LocalEditorInteractionScope provides interactionScope,
        LocalEditorUiState provides fixture.uiState,
      ) {
        EditorScreenLayout(
          state = remember { EditorScreenState(fixture.viewportState) },
          scrollFrame = fixture.scrollFrame,
          visibleArea = fixture.visibleArea,
          viewportScrollableState = viewportScrollableState,
          viewportContentWidth = HeaderFixtureContentWidth,
          viewportScrollReconcileMode = EditorViewportScrollReconcileMode.Disabled,
          onEditorPointerInput = { fixture.editorPointerInputCount += 1 },
          onMeasuredViewportSizeChange = {},
          header = {
            Box(
              Modifier.width(TestViewportSize.width.dp)
                .height(HeaderHeightPx.dp)
                .testTag(HeaderTag)
                .onGloballyPositioned { coordinates ->
                  fixture.headerPositionInRoot = coordinates.positionInRoot()
                }
            ) {
              BasicTextField(
                value = fixture.fieldValue,
                onValueChange = { fixture.fieldValue = it },
                modifier =
                  Modifier.width(240.dp).height(64.dp).testTag(HeaderFieldTag).onFocusChanged {
                    fixture.headerFocused = it.isFocused
                  },
              )
            }
          },
          body = {
            Box(
              Modifier.fillMaxWidth()
                .height(HeaderFixtureBodyHeight.dp)
                .testTag(BodyTag)
                .onGloballyPositioned { coordinates ->
                  fixture.bodyPositionInRoot = coordinates.positionInRoot()
                }
            )
          },
          viewportOverlay = viewportOverlay,
          toolbar = {},
          modifier =
            Modifier.size(width = TestViewportSize.width.dp, height = TestViewportSize.height.dp)
              .testTag(LayoutTag),
        )
      }
    }
    waitForIdle()
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
          body = { Box(Modifier.fillMaxWidth().height(800.dp)) },
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

  private fun assertMixedEditorSideSuppressed(fixture: HeaderInputFixture, expectedZoom: Float) {
    assertTrue(fixture.scrollDeltas.isEmpty())
    assertEquals(expectedZoom, fixture.zoomController.displayZoom)
    assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
    assertTrue(fixture.fake.enqueued.isEmpty())
  }

  private fun androidx.compose.ui.test.ComposeUiTest.assertFreshHeaderBodyPinchWorks(
    fixture: HeaderInputFixture
  ) {
    val zoomBeforePinch = fixture.zoomController.displayZoom
    val root = onNodeWithTag(LayoutTag)
    root.performTouchInput {
      down(pointerId = 0, position = Offset(x = 180f, y = 40f))
      down(pointerId = 1, position = Offset(x = 240f, y = HeaderHeightPx + 80f))
      updatePointerTo(pointerId = 0, position = Offset(x = 170f, y = 20f))
      updatePointerTo(pointerId = 1, position = Offset(x = 250f, y = HeaderHeightPx + 110f))
      move()
    }
    waitForIdle()

    assertEquals(
      EditorInteractionMode.ViewportZooming,
      fixture.interactionScope.controller.interactionMode,
    )
    assertTrue(fixture.zoomController.displayZoom > zoomBeforePinch)

    root.performTouchInput {
      up(pointerId = 1)
      up(pointerId = 0)
    }
    waitForIdle()
    assertEquals(EditorInteractionMode.Idle, fixture.interactionScope.controller.interactionMode)
  }

  @Composable
  private fun MixedPointerControl(onDrag: () -> Unit) {
    Box(
      Modifier.width(160.dp)
        .height(HeaderHeightPx.dp)
        .testTag(HeaderControlTag)
        .viewportDirectControl()
        .pointerInput(onDrag) {
          detectDragGestures { change, _ ->
            change.consume()
            onDrag()
          }
        }
    )
  }

  @OptIn(InternalComposeUiApi::class)
  private fun touchScenePointer(
    id: Long,
    position: Offset,
    pressed: Boolean = true,
  ): ComposeScenePointer =
    ComposeScenePointer(
      id = PointerId(id),
      position = position,
      pressed = pressed,
      type = PointerType.Touch,
    )

  @Composable
  private fun ViewportOverlayTarget() {
    Box(
      Modifier.fillMaxSize()
        .testTag(ViewportOverlayTag)
        .viewportDirectControl()
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

  private class HeaderInputFixture {
    val coroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
    val fake =
      FakeFfiEditor(
        pageSizesProvider = {
          listOf(PageSize(width = HeaderFixtureContentWidth, height = HeaderFixtureBodyHeight))
        }
      )
    val editor = Editor(fake, coroutineScope)
    val viewportState = EditorViewportState()
    val visibleArea = EditorVisibleArea(viewport = TestViewportSize)
    val scrollDeltas = mutableListOf<Offset>()
    val zoomController =
      EditorZoomController().apply {
        syncLayout(layoutSpec = HeaderFixtureLayoutSpec, viewportWidth = TestViewportSize.width)
      }
    val uiState =
      EditorUiState().apply {
        updateDisplayZoom(1f)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateInteractionSurfaceBounds(
          boundsInRoot =
            Rect(
              left = 0f,
              top = HeaderHeightPx,
              right = HeaderFixtureContentWidth,
              bottom = HeaderHeightPx + HeaderFixtureBodyHeight,
            ),
          density = 1f,
        )
        updateEditorBounds(
          boundsInRoot =
            Rect(
              left = 0f,
              top = HeaderHeightPx,
              right = HeaderFixtureContentWidth,
              bottom = HeaderHeightPx + HeaderFixtureBodyHeight,
            ),
          density = 1f,
        )
      }
    val viewportZoomConfig =
      EditorViewportZoomSemanticConfig(
        layoutSpec = HeaderFixtureLayoutSpec,
        zoomController = zoomController,
        viewportState = viewportState,
        uiState = uiState,
        pageSizes =
          listOf(PageSize(width = HeaderFixtureContentWidth, height = HeaderFixtureBodyHeight)),
        viewportWidth = TestViewportSize.width,
        density = 1f,
        onZoomSnap = {},
      )
    val layoutSpec: EditorDocumentLayoutSpec = HeaderFixtureLayoutSpec
    val scrollFrame =
      EditorScrollFrame(
        state = EditorState.Initial,
        layoutSpec = layoutSpec,
        displayZoom = 1f,
        visibleArea = visibleArea,
        autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea),
        headerHeight = HeaderHeightPx,
        density = 1f,
        editorBounds =
          EditorBoundsInContainer(
            x = 0f,
            y = 0f,
            width = HeaderFixtureContentWidth,
            height = HeaderFixtureBodyHeight,
          ),
      )
    lateinit var interactionScope: EditorInteractionScope
    var fieldValue by mutableStateOf(TextFieldValue(HeaderText, TextRange.Zero))
    var headerFocused = false
    var headerPositionInRoot: Offset? = null
    var bodyPositionInRoot: Offset? = null
    var editorPointerInputCount = 0

    fun close() {
      coroutineScope.cancel()
    }
  }

  private companion object {
    val HasScrollByAction =
      SemanticsMatcher("has ScrollBy action") { node ->
        node.config.contains(SemanticsActions.ScrollBy)
      }
    const val BodyTag = "editor-screen-layout-body"
    const val HeaderControlTag = "editor-screen-layout-header-control"
    const val HeaderFieldTag = "editor-screen-layout-header-field"
    const val HeaderTag = "editor-screen-layout-header"
    const val LayoutTag = "editor-screen-layout"
    const val SubPaneTag = "editor-screen-layout-subpane"
    const val ViewportOverlayTag = "editor-screen-layout-viewport-overlay-target"
    const val HeaderFixtureBodyHeight = 800f
    const val HeaderFixtureContentWidth = 640f
    const val HeaderHeightPx = 96f
    const val HeaderText = "Header title"
    val ControlPointerMove = Offset(x = 40f, y = 40f)
    val ControlPointerStart = Offset(x = 80f, y = 40f)
    val EditorPointerMove = Offset(x = 300f, y = HeaderHeightPx + 140f)
    val EditorPointerStart = Offset(x = 280f, y = 40f)
    val EditorSurvivorMove = Offset(x = 300f, y = HeaderHeightPx + 220f)
    val HeaderFixtureLayoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = HeaderFixtureContentWidth,
        pageHeight = HeaderFixtureBodyHeight,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      )
    val TestViewportSize = Size(width = 320f, height = 640f)
  }
}
