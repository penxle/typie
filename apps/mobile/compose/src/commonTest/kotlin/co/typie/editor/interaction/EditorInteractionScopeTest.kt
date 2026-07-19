package co.typie.editor.interaction

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import co.typie.editor.Editor
import co.typie.editor.EditorState
import co.typie.editor.FakeFfiEditor
import co.typie.editor.body.EditorDocumentLayoutSpec
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.viewport.EditorViewportState
import co.typie.ext.ScrollGestureLockState
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EditorInteractionScopeTest {
  @Test
  fun `editor state observation is ignored before editor attaches`() =
    runTest(StandardTestDispatcher()) {
      val scope = EditorInteractionScope(coroutineScope = this)

      scope.update(
        editor = null,
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        uiState = EditorUiState(),
        density = 1f,
        visibleArea = EditorVisibleArea(),
        viewportState = EditorViewportState(),
        scrollGestureLockState = ScrollGestureLockState(),
        viewportZoomConfig = null,
        onSelectionHaptic = {},
        onRequestSoftwareKeyboard = {},
      )

      scope.onEditorStateChanged(EditorState.Initial)
    }

  @Test
  fun `root Down eligibility and mapping distinguish header from document body`() =
    runTest(StandardTestDispatcher()) {
      val uiState =
        EditorUiState().apply {
          updateInteractionSurfaceBounds(
            boundsInRoot = Rect(left = 0f, top = 120f, right = 400f, bottom = 920f),
            density = 1f,
          )
          updateEditorBounds(
            boundsInRoot = Rect(left = 40f, top = 200f, right = 360f, bottom = 680f),
            density = 1f,
          )
        }
      val scope = EditorInteractionScope(coroutineScope = this)
      scope.update(
        editor = Editor(FakeFfiEditor(), this, StandardTestDispatcher(testScheduler)),
        bringIntoViewRequests = EditorBringIntoViewRequests(),
        uiState = uiState,
        density = 1f,
        visibleArea = EditorVisibleArea(),
        viewportState = EditorViewportState(),
        scrollGestureLockState = ScrollGestureLockState(),
        viewportZoomConfig = null,
        layoutSpec = EditorDocumentLayoutSpec.Continuous(maxWidth = 320f),
        onSelectionHaptic = {},
        onRequestSoftwareKeyboard = {},
      )

      val headerPositionInRoot = Offset(x = 80f, y = 100f)
      val bodyPositionInRoot = Offset(x = 80f, y = 240f)

      // Down admission is decided in root coordinates. The cross-boundary
      // headerFieldPanClaimsViewportWithoutPromotingReleaseToFieldOrEditor layout test protects
      // this classification from being promoted after the pointer moves into the body.
      assertFalse(scope.containsDocumentInteraction(headerPositionInRoot))
      assertTrue(scope.containsDocumentInteraction(bodyPositionInRoot))
      assertFalse(
        scope.isTapEligible(headerPositionInRoot),
        "header root Down must not be document-eligible",
      )
      assertTrue(
        scope.isTapEligible(bodyPositionInRoot),
        "body root Down must remain document-eligible",
      )

      val bodyMapped = assertNotNull(scope.resolveInteractionPosition(bodyPositionInRoot))
      assertEquals(
        Offset(x = 40f, y = 40f),
        bodyMapped,
        "body mapping must subtract the editor rect top-left in root",
      )

      val headerMapped = assertNotNull(scope.resolveInteractionPosition(headerPositionInRoot))
      assertEquals(Offset(x = 40f, y = -100f), headerMapped)
      assertTrue(headerMapped.y < 0f, "mapping above the body must stay valid and negative")
    }
}
