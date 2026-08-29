package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.PointerEvent
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.PointerInputModifierNode
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.click
import androidx.compose.ui.test.hasClickAction
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorZoomController
import co.typie.editor.EditorZoomLandmark
import co.typie.editor.LocalEditorZoomController
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.screen.editor.editor.header.EditorHeaderReadingTapTracker
import co.typie.screen.editor.editor.header.observeEditorHeaderReadingTaps
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorZoomOverlayDesktopTest {
  @Test
  fun hiddenTouchOnlyZoomControlLeavesTheHitTestTree() = runComposeUiTest {
    mainClock.autoAdvance = false
    val fixture = ZoomOverlayFixture()
    setContent { fixture.Content() }
    mainClock.advanceTimeByFrame()
    runOnIdle {
      fixture.state.expireLandmark(fixture.state.landmarkRequest)
      fixture.state.expireVisibility(fixture.state.visibilityRequest)
    }
    mainClock.advanceTimeBy(200)

    onNodeWithTag(OverlayTag).assertDoesNotExist()
  }

  @Test
  fun zoomControlClaimsHeaderTapsOnlyWhileVisible() = runComposeUiTest {
    var toggleCount = 0
    var headerActivationCount = 0
    val fixture = ZoomOverlayFixture()
    val headerTapTracker =
      EditorHeaderReadingTapTracker(
        touchSlopPx = 8f,
        maxTapDistancePx = 20f,
        doubleTapToEditEnabled = false,
      )
    setContent {
      CompositionLocalProvider(
        LocalEditorZoomController provides fixture.controller,
        LocalAppColors provides LightColors,
      ) {
        Box(Modifier.size(240.dp)) {
          Box(
            Modifier.fillMaxSize()
              .testTag(SurfaceTag)
              .observeEditorHeaderReadingTaps(
                enabled = { true },
                tracker = headerTapTracker,
                currentSelectionIsExpanded = { false },
                offsetForPosition = { 0 },
                onActivate = { headerActivationCount += 1 },
                onShowHint = {},
              )
          )
          Box(Modifier.fillMaxSize().sharePointerInputWithSiblingsForTest()) {
            EditorZoomOverlay(
              state = fixture.state,
              nonTouchPointerActive = false,
              onZoomOut = { false },
              onZoomIn = { false },
              onToggleZoom = {
                toggleCount += 1
                null
              },
              onRequestEditorFocus = {},
              modifier = Modifier.align(Alignment.Center).testTag(OverlayTag),
            )
          }
        }
      }
    }

    onNodeWithTag(OverlayTag).performTouchInput { click() }

    runOnIdle {
      assertEquals(1, toggleCount)
      assertEquals(0, headerActivationCount)
    }

    runOnIdle { fixture.controller.setDisplayZoom(1f, fixture.layoutSpec, fixture.viewportWidth) }
    waitForIdle()
    runOnIdle {
      fixture.state.expireLandmark(fixture.state.landmarkRequest)
      fixture.state.expireVisibility(fixture.state.visibilityRequest)
    }

    onNodeWithTag(SurfaceTag).performTouchInput { click() }

    runOnIdle {
      assertEquals(1, toggleCount)
      assertEquals(1, headerActivationCount)
    }
  }

  @Test
  fun zoomControlsRequestEditorFocusAfterEveryAction() = runComposeUiTest {
    var toggleCount = 0
    var focusRequestCount = 0
    val fixture =
      ZoomOverlayFixture(
        pointerCapable = true,
        onToggleZoom = { toggleCount += 1 },
        onRequestEditorFocus = { focusRequestCount += 1 },
      )
    setContent { fixture.Content() }

    onAllNodes(hasClickAction()).apply {
      repeat(fetchSemanticsNodes().size) { index -> this[index].performTouchInput { click() } }
    }

    runOnIdle { assertEquals(1, toggleCount) }
    runOnIdle { assertEquals(3, focusRequestCount) }
  }

  @Test
  fun plainZoomActivityHidesAfterOneSecond() = runComposeUiTest {
    mainClock.autoAdvance = false
    val fixture = ZoomOverlayFixture()
    setContent { fixture.Content() }
    mainClock.advanceTimeByFrame()

    mainClock.advanceTimeBy(900)
    runOnIdle { assertTrue(fixture.state.visible) }

    mainClock.advanceTimeBy(200)
    runOnIdle { assertFalse(fixture.state.visible) }
  }

  @Test
  fun landmarkReturnsToPercentageAfterOneSecondAndHidesAfterTwoSeconds() = runComposeUiTest {
    mainClock.autoAdvance = false
    val fixture = ZoomOverlayFixture()
    setContent { fixture.Content() }
    mainClock.advanceTimeByFrame()
    runOnIdle { fixture.state.expireVisibility(fixture.state.visibilityRequest) }

    runOnIdle { fixture.controller.setDisplayZoom(1f, fixture.layoutSpec, fixture.viewportWidth) }
    mainClock.advanceTimeByFrame()
    runOnIdle { assertEquals(EditorZoomLandmark.Unit, fixture.state.announcedLandmark) }

    mainClock.advanceTimeBy(900)
    runOnIdle { assertEquals(EditorZoomLandmark.Unit, fixture.state.announcedLandmark) }
    runOnIdle { assertTrue(fixture.state.visible) }

    mainClock.advanceTimeBy(200)
    runOnIdle { assertEquals(null, fixture.state.announcedLandmark) }
    runOnIdle { assertTrue(fixture.state.visible) }

    mainClock.advanceTimeBy(700)
    runOnIdle { assertTrue(fixture.state.visible) }

    mainClock.advanceTimeBy(300)
    runOnIdle { assertFalse(fixture.state.visible) }
  }

  private class ZoomOverlayFixture(
    private val pointerCapable: Boolean = false,
    private val onToggleZoom: () -> Unit = {},
    private val onRequestEditorFocus: () -> Unit = {},
  ) {
    val state = EditorZoomIndicatorState()
    val controller = EditorZoomController()
    val layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 400f)
    val viewportWidth = 500f

    init {
      controller.syncLayout(layoutSpec = layoutSpec, viewportWidth = viewportWidth)
      controller.setDisplayZoom(1.2f, layoutSpec = layoutSpec, viewportWidth = viewportWidth)
      if (pointerCapable) state.onPanePointerEnter(landmark = null)
    }

    @androidx.compose.runtime.Composable
    fun Content() {
      CompositionLocalProvider(
        LocalEditorZoomController provides controller,
        LocalAppColors provides LightColors,
      ) {
        Box(Modifier.size(240.dp)) {
          EditorZoomOverlay(
            state = state,
            nonTouchPointerActive = pointerCapable,
            onZoomOut = { false },
            onZoomIn = { false },
            onToggleZoom = {
              onToggleZoom()
              null
            },
            onRequestEditorFocus = onRequestEditorFocus,
            modifier = Modifier.testTag(OverlayTag),
          )
        }
      }
    }
  }
}

private const val OverlayTag = "editor-zoom-overlay"
private const val SurfaceTag = "editor-surface"

private data object SharePointerInputWithSiblingsForTestElement :
  ModifierNodeElement<SharePointerInputWithSiblingsForTestNode>() {
  override fun create(): SharePointerInputWithSiblingsForTestNode =
    SharePointerInputWithSiblingsForTestNode()

  override fun update(node: SharePointerInputWithSiblingsForTestNode) = Unit
}

private class SharePointerInputWithSiblingsForTestNode : Modifier.Node(), PointerInputModifierNode {
  override fun sharePointerInputWithSiblings(): Boolean = true

  override fun onPointerEvent(pointerEvent: PointerEvent, pass: PointerEventPass, bounds: IntSize) =
    Unit

  override fun onCancelPointerInput() = Unit
}

private fun Modifier.sharePointerInputWithSiblingsForTest(): Modifier =
  this then SharePointerInputWithSiblingsForTestElement
