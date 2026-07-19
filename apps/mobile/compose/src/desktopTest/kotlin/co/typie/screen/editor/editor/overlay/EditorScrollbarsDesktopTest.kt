package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.SubcomposeLayout
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotDisplayed
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState
import co.typie.ui.theme.LightColors
import co.typie.ui.theme.LocalAppColors
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@OptIn(ExperimentalTestApi::class)
class EditorScrollbarsDesktopTest {
  @Test
  fun firstZoomedThumbPlacementUsesTheNewMeasuredExtent() = runComposeUiTest {
    val viewportState = EditorViewportState()
    val contentSize = mutableStateOf(Size(width = 100f, height = 200f))
    val zoomedPlacements = mutableListOf<Rect>()
    var recordZoomedPlacements = false

    setContent {
      ScrollbarLayoutFrame(viewportState = viewportState, contentSize = contentSize.value) {
        EditorScrollbarThumbLayout(
          horizontal = false,
          viewportState = viewportState,
          visibleArea = VisibleArea,
          modifier = Modifier.fillMaxSize(),
        ) {
          Box(
            Modifier.onPlaced { coordinates ->
              if (recordZoomedPlacements) {
                zoomedPlacements += coordinates.boundsInRoot()
              }
            }
          )
        }
      }
    }
    waitForIdle()

    runOnIdle {
      recordZoomedPlacements = true
      viewportState.scrollToTransformTarget(
        offset = Offset(x = 0f, y = 100f),
        retainUntilMeasuredBounds = true,
      )
      contentSize.value = Size(width = 100f, height = 300f)
    }
    waitForIdle()

    assertTrue(zoomedPlacements.isNotEmpty())
    zoomedPlacements.forEach { placement ->
      assertEquals(34f, placement.top, absoluteTolerance = 0.01f)
      assertEquals(32f, placement.height, absoluteTolerance = 0.01f)
    }
  }

  @Test
  fun remeasurementResolvesBothAxisReservationsFromOneFrame() = runComposeUiTest {
    val viewportState = EditorViewportState()
    val contentSize = mutableStateOf(Size(width = 100f, height = 100f))
    val verticalPlacements = mutableListOf<Rect>()
    val horizontalPlacements = mutableListOf<Rect>()
    var recordPlacements = false

    setContent {
      ScrollbarLayoutFrame(viewportState = viewportState, contentSize = contentSize.value) {
        EditorScrollbarThumbLayout(
          horizontal = false,
          viewportState = viewportState,
          visibleArea = VisibleArea,
          modifier = Modifier.fillMaxSize(),
        ) {
          Box(
            Modifier.testTag(VerticalThumbTag).onPlaced { coordinates ->
              if (recordPlacements) {
                verticalPlacements += coordinates.boundsInRoot()
              }
            }
          )
        }
        EditorScrollbarThumbLayout(
          horizontal = true,
          viewportState = viewportState,
          visibleArea = VisibleArea,
          modifier = Modifier.fillMaxSize(),
        ) {
          Box(
            Modifier.testTag(HorizontalThumbTag).onPlaced { coordinates ->
              if (recordPlacements) {
                horizontalPlacements += coordinates.boundsInRoot()
              }
            }
          )
        }
      }
    }
    waitForIdle()
    onNodeWithTag(VerticalThumbTag).assertIsNotDisplayed()
    onNodeWithTag(HorizontalThumbTag).assertIsNotDisplayed()

    runOnIdle {
      recordPlacements = true
      contentSize.value = Size(width = 200f, height = 200f)
    }
    waitForIdle()
    onNodeWithTag(VerticalThumbTag).assertIsDisplayed()
    onNodeWithTag(HorizontalThumbTag).assertIsDisplayed()

    assertTrue(verticalPlacements.isNotEmpty())
    assertTrue(horizontalPlacements.isNotEmpty())
    verticalPlacements.forEach { placement ->
      assertEquals(2f, placement.top, absoluteTolerance = 0.01f)
      assertEquals(42f, placement.height, absoluteTolerance = 0.01f)
    }
    horizontalPlacements.forEach { placement ->
      assertEquals(2f, placement.left, absoluteTolerance = 0.01f)
      assertEquals(42f, placement.width, absoluteTolerance = 0.01f)
    }

    runOnIdle { contentSize.value = Size(width = 100f, height = 100f) }
    waitForIdle()

    onNodeWithTag(VerticalThumbTag).assertIsNotDisplayed()
    onNodeWithTag(HorizontalThumbTag).assertIsNotDisplayed()
  }

  @Test
  fun hiddenOverflowingAxisDoesNotReserveTrackSpace() = runComposeUiTest {
    val viewportState = EditorViewportState()
    val horizontalPlacements = mutableListOf<Rect>()
    val visibleArea = VisibleArea.copy(topInset = 70f)

    setContent {
      ScrollbarLayoutFrame(
        viewportState = viewportState,
        contentSize = Size(width = 200f, height = 200f),
      ) {
        EditorScrollbarThumbLayout(
          horizontal = false,
          viewportState = viewportState,
          visibleArea = visibleArea,
          modifier = Modifier.fillMaxSize(),
        ) {
          Box(Modifier.testTag(VerticalThumbTag))
        }
        EditorScrollbarThumbLayout(
          horizontal = true,
          viewportState = viewportState,
          visibleArea = visibleArea,
          modifier = Modifier.fillMaxSize(),
        ) {
          Box(
            Modifier.testTag(HorizontalThumbTag).onPlaced { coordinates ->
              horizontalPlacements += coordinates.boundsInRoot()
            }
          )
        }
      }
    }
    waitForIdle()

    onNodeWithTag(VerticalThumbTag).assertIsNotDisplayed()
    onNodeWithTag(HorizontalThumbTag).assertIsDisplayed()
    assertTrue(horizontalPlacements.isNotEmpty())
    horizontalPlacements.forEach { placement ->
      assertEquals(48f, placement.width, absoluteTolerance = 0.01f)
    }
  }

  @Test
  fun dragRebasesWhenMeasuredExtentChanges() = runComposeUiTest {
    val viewportState = EditorViewportState()
    val contentSize = mutableStateOf(Size(width = 100f, height = 200f))

    setContent {
      CompositionLocalProvider(LocalAppColors provides LightColors) {
        ScrollbarLayoutFrame(viewportState = viewportState, contentSize = contentSize.value) {
          EditorScrollbars(
            viewportState = viewportState,
            visibleArea = VisibleArea,
            layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 100f),
            pageSizes = emptyList(),
            displayZoom = 1f,
            modifier = Modifier.fillMaxSize(),
          )
        }
      }
    }
    waitForIdle()

    runOnIdle { viewportState.scrollToY(1f) }
    waitForIdle()

    onRoot().performTouchInput {
      down(Offset(x = 90f, y = 20f))
      moveBy(Offset(x = 0f, y = 10f))
    }
    waitForIdle()
    assertEquals(21.833334f, viewportState.scrollOffset.y, absoluteTolerance = 0.01f)

    runOnIdle { contentSize.value = Size(width = 100f, height = 300f) }
    waitForIdle()

    onRoot().performTouchInput {
      moveBy(Offset(x = 0f, y = 10f))
      up()
    }
    waitForIdle()

    assertEquals(53.083336f, viewportState.scrollOffset.y, absoluteTolerance = 0.01f)
  }

  @Test
  fun firstZoomedIndicatorPlacementUsesTheNewThumbGeometry() = runComposeUiTest {
    val viewportState = EditorViewportState()
    val contentSize = mutableStateOf(Size(width = 100f, height = 200f))
    val zoomedPlacements = mutableListOf<Rect>()
    var recordZoomedPlacements = false

    setContent {
      ScrollbarLayoutFrame(viewportState = viewportState, contentSize = contentSize.value) {
        EditorScrollbarIndicatorLayout(
          viewportState = viewportState,
          visibleArea = VisibleArea,
          modifier = Modifier.fillMaxSize(),
        ) {
          Box(
            Modifier.size(width = 20.dp, height = 24.dp).onPlaced { coordinates ->
              if (recordZoomedPlacements) {
                zoomedPlacements += coordinates.boundsInRoot()
              }
            }
          )
        }
      }
    }
    waitForIdle()

    runOnIdle {
      recordZoomedPlacements = true
      viewportState.scrollToTransformTarget(
        offset = Offset(x = 0f, y = 100f),
        retainUntilMeasuredBounds = true,
      )
      contentSize.value = Size(width = 100f, height = 300f)
    }
    waitForIdle()

    assertTrue(zoomedPlacements.isNotEmpty())
    zoomedPlacements.forEach { placement ->
      assertEquals(60f, placement.left, absoluteTolerance = 0.01f)
      assertEquals(38f, placement.top, absoluteTolerance = 0.01f)
    }
  }

  private companion object {
    const val HorizontalThumbTag = "horizontal-scrollbar-thumb"
    const val VerticalThumbTag = "vertical-scrollbar-thumb"
    val VisibleArea = EditorVisibleArea(viewport = Size(width = 100f, height = 100f))
  }
}

@Composable
private fun ScrollbarLayoutFrame(
  viewportState: EditorViewportState,
  contentSize: Size,
  overlay: @Composable () -> Unit,
) {
  CompositionLocalProvider(LocalDensity provides Density(1f)) {
    SubcomposeLayout(Modifier.size(100.dp)) { constraints ->
      viewportState.updateMeasuredBounds(
        viewportSize = Size(width = 100f, height = 100f),
        contentSize = contentSize,
      )
      val overlayPlaceables =
        subcompose(ScrollbarLayoutSlot.Overlay, overlay).map { it.measure(constraints) }

      layout(width = constraints.maxWidth, height = constraints.maxHeight) {
        overlayPlaceables.forEach { it.place(x = 0, y = 0) }
      }
    }
  }
}

private enum class ScrollbarLayoutSlot {
  Overlay
}
