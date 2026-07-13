package co.typie.editor.interaction

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.EditorZoomController
import co.typie.editor.PagePoint
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.ffi.Size as PageSize
import co.typie.editor.interaction.semantics.EditorViewportZoomSemanticConfig
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.viewport.EditorViewportState
import kotlin.test.Test
import kotlin.test.assertEquals

@OptIn(ExperimentalTestApi::class)
class EditorInteractionsDesktopTest {
  @Test
  fun `pinch routing samples once per pointer event frame`() = runComposeUiTest {
    val fixture = Fixture()
    setEditorContent(fixture)

    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 0, position = Offset(100f, 100f))
    }
    onNodeWithTag(EditorTag).performTouchInput {
      down(pointerId = 1, position = Offset(200f, 100f))
    }
    assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

    val revisionBeforeMove = fixture.viewportState.lastScrollRevision
    onNodeWithTag(EditorTag).performTouchInput {
      updatePointerTo(pointerId = 0, position = Offset(75f, 100f))
      updatePointerTo(pointerId = 1, position = Offset(225f, 100f))
      move()
    }
    assertEquals(revisionBeforeMove + 1, fixture.viewportState.lastScrollRevision)

    onNodeWithTag(EditorTag).performTouchInput {
      up(pointerId = 1)
      up(pointerId = 0)
    }
  }

  @Test
  fun `third pinch pointer cancels and suppresses restart until all pointers are up`() =
    runComposeUiTest {
      val fixture = Fixture()
      setEditorContent(fixture)

      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
      }
      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 1, position = Offset(200f, 100f))
      }
      assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 2, position = Offset(150f, 200f))
      }
      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput { up(pointerId = 2) }
      onNodeWithTag(EditorTag).performTouchInput {
        updatePointerTo(pointerId = 0, position = Offset(70f, 100f))
        updatePointerTo(pointerId = 1, position = Offset(230f, 100f))
        move()
      }
      assertEquals(EditorInteractionMode.Idle, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 0, position = Offset(100f, 100f))
      }
      onNodeWithTag(EditorTag).performTouchInput {
        down(pointerId = 1, position = Offset(200f, 100f))
      }
      assertEquals(EditorInteractionMode.ViewportZooming, fixture.controller.interactionMode)

      onNodeWithTag(EditorTag).performTouchInput {
        up(pointerId = 1)
        up(pointerId = 0)
      }
    }

  private fun androidx.compose.ui.test.ComposeUiTest.setEditorContent(fixture: Fixture) {
    setContent {
      Box(
        Modifier.size(400.dp)
          .testTag(EditorTag)
          .editorInteractions(
            density = 1f,
            interactionController = fixture.controller,
            coordinateResolver = IdentityCoordinateResolver,
          )
      )
    }
    waitForIdle()
  }

  private class Fixture {
    val viewportState =
      EditorViewportState().apply {
        updateMeasuredBounds(
          viewportSize = Size(width = 400f, height = 400f),
          contentSize = Size(width = 2000f, height = 2000f),
        )
      }
    private val layoutSpec =
      EditorDocumentLayoutSpec.Paginated(
        pageWidth = 720f,
        pageHeight = 960f,
        pageMarginTop = 0f,
        pageMarginBottom = 0f,
        pageMarginLeft = 0f,
        pageMarginRight = 0f,
      )
    private val pageSizes = listOf(PageSize(width = 720f, height = 960f))
    private val zoomController = EditorZoomController()
    private val uiState =
      EditorUiState().apply {
        updateDisplayZoom(1f)
        updatePageOffset(page = 0, offset = Offset.Zero)
        updateEditorBounds(
          boundsInRoot = Rect(left = 0f, top = 0f, right = 400f, bottom = 400f),
          density = 1f,
        )
      }
    private val host = TestHost()
    private val semantics =
      EditorInteractionSemantics(effects = host).apply {
        zoomController.syncLayout(layoutSpec = layoutSpec, viewportWidth = 400f)
        viewportZoom.configure(
          EditorViewportZoomSemanticConfig(
            layoutSpec = layoutSpec,
            zoomController = zoomController,
            viewportState = viewportState,
            uiState = uiState,
            pageSizes = pageSizes,
            viewportWidth = 400f,
            density = 1f,
            onZoomSnap = {},
          )
        )
      }
    val controller =
      EditorInteractionController(
        editorProvider = { error("Pinch routing must not access the editor") },
        effects = host,
        geometry = host,
        semantics = semantics,
        uiStateProvider = { uiState },
      )
  }

  private class TestHost : EditorInteractionEffects, EditorInteractionGeometry {
    override val density: Float = 1f

    override fun resolvePoint(positionInNode: Offset): PagePoint? = null

    override fun resolvePagePosition(page: Int, x: Float, y: Float): Offset? = null

    override fun resolveEdgeAutoScrollViewport(): EditorEdgeAutoScrollViewport? = null

    override fun dispatchEdgeAutoScroll(delta: Offset): Offset = Offset.Zero

    override fun scheduleTapDispatch(dispatchAtMillis: Long) = Unit

    override fun cancelTapDispatch() = Unit

    override fun scheduleLongPressDispatch(
      pointerId: Long,
      position: Offset,
      dispatchAtMillis: Long,
    ) = Unit

    override fun cancelLongPressDispatch() = Unit

    override fun launchInteraction(block: suspend () -> Unit) = Unit

    override fun requestFocus(editor: Editor): Boolean = false

    override fun requestSoftwareKeyboard() = Unit

    override fun enqueuePointerCancel() = Unit

    override fun setScrollGestureLocked(locked: Boolean) = Unit

    override fun performSelectionHaptic() = Unit

    override fun requestCurrentSelectionHead(version: Long) = Unit
  }

  private data object IdentityCoordinateResolver : EditorPointerCoordinateResolver {
    override fun positionInRoot(position: Offset): Offset = position

    override fun positionForPointerStart(position: Offset): Offset = position

    override fun positionForTapStart(position: Offset): Offset? = null

    override fun positionForActivePointer(position: Offset): Offset = position
  }

  private companion object {
    const val EditorTag = "editor-interactions"
  }
}
