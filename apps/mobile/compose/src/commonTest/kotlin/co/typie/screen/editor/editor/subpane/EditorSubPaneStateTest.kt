package co.typie.screen.editor.editor.subpane

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditorSubPaneStateTest {
  @Test
  fun `open activates related notes and dismiss clears it`() {
    val state = EditorSubPaneState()

    state.open(EditorSubPaneKey.RelatedNotes)

    assertTrue(state.isActive(EditorSubPaneKey.RelatedNotes))

    state.dismiss()

    assertNull(state.activeKey)
    assertNull(state.layoutInfo)
    assertFalse(state.isActive(EditorSubPaneKey.RelatedNotes))
  }

  @Test
  fun `resizable sub pane resizes editor until it reaches expanded height`() {
    assertEquals(
      EditorSubPaneVisibleAreaMode.ResizeEditor,
      resolveResizableSubPaneVisibleAreaMode(sheetHeight = 360f, expandedHeight = 720f),
    )
    assertEquals(
      EditorSubPaneVisibleAreaMode.OverlayEditor,
      resolveResizableSubPaneVisibleAreaMode(sheetHeight = 719.7f, expandedHeight = 720f),
    )
  }

  @Test
  fun `resizable sheet geometry reports layout values in dp`() {
    val geometry =
      resolveEditorResizableSheetGeometry(
        sheetHeightPx = 720f,
        expandedHeightPx = 1200f,
        keyboardOcclusionPx = 80f,
        visibility = 1f,
        density = 2f,
      )

    assertEquals(360f, geometry.sheetHeight)
    assertEquals(600f, geometry.expandedHeight)
    assertEquals(360f, geometry.visibleHeight)
  }

  @Test
  fun `resizable sheet geometry reports animated visible height`() {
    val geometry =
      resolveEditorResizableSheetGeometry(
        sheetHeightPx = 720f,
        expandedHeightPx = 1200f,
        keyboardOcclusionPx = 80f,
        visibility = 0.25f,
        density = 2f,
      )

    assertEquals(90f, geometry.visibleHeight)
  }

  @Test
  fun `resizable sheet geometry includes keyboard occlusion when it is taller than sheet`() {
    val geometry =
      resolveEditorResizableSheetGeometry(
        sheetHeightPx = 360f,
        expandedHeightPx = 1200f,
        keyboardOcclusionPx = 640f,
        visibility = 1f,
        density = 2f,
      )

    assertEquals(320f, geometry.visibleHeight)
  }

  @Test
  fun `keyboard aware minimum height preserves visible sheet area without exceeding expanded height`() {
    assertEquals(
      460f,
      resolveKeyboardAwareSheetMinHeight(
        minHeightPx = 240f,
        keyboardOcclusionPx = 300f,
        minKeyboardVisibleHeightPx = 160f,
        expandedHeightPx = 720f,
      ),
    )
    assertEquals(
      720f,
      resolveKeyboardAwareSheetMinHeight(
        minHeightPx = 240f,
        keyboardOcclusionPx = 640f,
        minKeyboardVisibleHeightPx = 160f,
        expandedHeightPx = 720f,
      ),
    )
  }

  @Test
  fun `resize mode contributes bottom occlusion and overlay mode does not`() {
    assertEquals(
      240f,
      resolveSubPaneBottomOcclusion(
        EditorSubPaneLayoutInfo(
          key = EditorSubPaneKey.RelatedNotes,
          visibleHeight = 240f,
          visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
        )
      ),
    )
    assertEquals(
      0f,
      resolveSubPaneBottomOcclusion(
        EditorSubPaneLayoutInfo(
          key = EditorSubPaneKey.RelatedNotes,
          visibleHeight = 640f,
          visibleAreaMode = EditorSubPaneVisibleAreaMode.OverlayEditor,
        )
      ),
    )
  }

  @Test
  fun `stale layout info from another surface is ignored`() {
    val state = EditorSubPaneState()
    state.open(EditorSubPaneKey.RelatedNotes)

    state.updateLayoutInfo(
      EditorSubPaneLayoutInfo(
        key = EditorSubPaneKey.Spellcheck,
        visibleHeight = 240f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      )
    )

    assertNull(state.layoutInfo)
  }

  @Test
  fun `layout clear from another surface is ignored`() {
    val state = EditorSubPaneState()
    val layoutInfo =
      EditorSubPaneLayoutInfo(
        key = EditorSubPaneKey.RelatedNotes,
        visibleHeight = 240f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      )
    state.open(EditorSubPaneKey.RelatedNotes)
    state.updateLayoutInfo(layoutInfo)

    state.clearLayoutInfo(EditorSubPaneKey.Spellcheck)

    assertEquals(layoutInfo, state.layoutInfo)
  }

  @Test
  fun `stale layout clear from previous surface does not clear active surface`() {
    val state = EditorSubPaneState()
    val layoutInfo =
      EditorSubPaneLayoutInfo(
        key = EditorSubPaneKey.Spellcheck,
        visibleHeight = 180f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      )
    state.open(EditorSubPaneKey.RelatedNotes)
    state.open(EditorSubPaneKey.Spellcheck)
    state.updateLayoutInfo(layoutInfo)

    state.clearLayoutInfo(EditorSubPaneKey.RelatedNotes)

    assertEquals(layoutInfo, state.layoutInfo)
  }
}
