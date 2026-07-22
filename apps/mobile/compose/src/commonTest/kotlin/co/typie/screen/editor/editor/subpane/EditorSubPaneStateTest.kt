package co.typie.screen.editor.editor.subpane

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class EditorSubPaneStateTest {
  @Test
  fun `open activates related notes and dismiss clears it`() {
    val state = EditorSubPaneState()

    state.open(EditorSubPane.RelatedNotes)

    assertTrue(state.isActive(EditorSubPane.RelatedNotes))

    state.dismiss()

    assertNull(state.active)
    assertNull(state.layoutInfo)
    assertFalse(state.isActive(EditorSubPane.RelatedNotes))
  }

  @Test
  fun `open can store a table axis actions payload`() {
    val state = EditorSubPaneState()
    val pane =
      EditorSubPane.TableAxisActions(
        target = tableAxisTarget(tableId = "table", index = 1),
        openedSelection = selection(offset = 1),
      )

    state.open(pane)

    assertEquals(pane, state.active)
    assertTrue(state.isActive(pane))
  }

  @Test
  fun `table axis actions remain valid while selection stays at opening selection`() {
    val openingSelection = selection(offset = 1)
    val pane =
      EditorSubPane.TableAxisActions(
        target = tableAxisTarget(tableId = "table", index = 1),
        openedSelection = openingSelection,
      )
    val state = EditorSubPaneState()
    state.open(pane)

    state.dismissTableAxisActionsIfSelectionChanged(openingSelection)

    assertEquals(pane, state.active)
    assertEquals(0, state.dismissRequestVersion)
    assertTrue(state.editorInputBlocked)
  }

  @Test
  fun `table axis actions become invalid when selection changes`() {
    val pane =
      EditorSubPane.TableAxisActions(
        target = tableAxisTarget(tableId = "table", index = 1),
        openedSelection = selection(offset = 1),
      )
    val state = EditorSubPaneState()
    state.open(pane)

    state.dismissTableAxisActionsIfSelectionChanged(selection(offset = 2))

    assertEquals(pane, state.active)
    assertEquals(1, state.dismissRequestVersion)
    assertFalse(state.editorInputBlocked)
  }

  @Test
  fun `no table axis actions pane remains valid for selection changes`() {
    val state = EditorSubPaneState()

    state.dismissTableAxisActionsIfSelectionChanged(selection(offset = 1))

    assertNull(state.active)
    assertEquals(0, state.dismissRequestVersion)
  }

  @Test
  fun `requesting dismiss keeps active pane until surface reports dismissed`() {
    val state = EditorSubPaneState()
    val pane = tableAxisPane(tableId = "table", index = 1)
    state.open(pane)

    assertTrue(state.editorInputBlocked)

    state.requestDismiss()

    assertEquals(pane, state.active)
    assertEquals(1, state.dismissRequestVersion)
    assertFalse(state.editorInputBlocked)

    state.dismiss()

    assertNull(state.active)
  }

  @Test
  fun `beginning dismiss releases editor input while pane remains active`() {
    val state = EditorSubPaneState()
    val pane = tableAxisPane(tableId = "table", index = 1)
    state.open(pane)

    state.beginDismiss()

    assertEquals(pane, state.active)
    assertFalse(state.editorInputBlocked)
    assertEquals(0, state.dismissRequestVersion)
  }

  @Test
  fun `cancelling user dismissal blocks editor input again`() {
    val state = EditorSubPaneState()
    val pane = tableAxisPane(tableId = "table", index = 1)
    state.open(pane)
    state.beginDismiss()

    state.cancelDismiss()

    assertEquals(pane, state.active)
    assertTrue(state.editorInputBlocked)
    assertEquals(0, state.dismissRequestVersion)
  }

  @Test
  fun `cancelling requested dismissal keeps input released and requests dismissal again`() {
    val state = EditorSubPaneState()
    val pane = tableAxisPane(tableId = "table", index = 1)
    state.open(pane)
    state.requestDismiss()

    state.cancelDismiss()

    assertEquals(pane, state.active)
    assertFalse(state.editorInputBlocked)
    assertEquals(2, state.dismissRequestVersion)
  }

  @Test
  fun `opening pane after dismissal starts blocks editor input again`() {
    val state = EditorSubPaneState()
    state.open(tableAxisPane(tableId = "table", index = 0))
    state.beginDismiss()

    val nextPane = tableAxisPane(tableId = "table", index = 1)
    state.open(nextPane)

    assertEquals(nextPane, state.active)
    assertTrue(state.editorInputBlocked)
    assertEquals(0, state.dismissRequestVersion)
  }

  @Test
  fun `reopening the same pane keeps layout info`() {
    val state = EditorSubPaneState()
    val pane = tableAxisPane(tableId = "table", index = 1)
    val layoutInfo =
      EditorSubPaneLayoutInfo(
        pane = pane,
        visibleHeight = 180f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      )
    state.open(pane)
    state.updateLayoutInfo(layoutInfo)

    state.open(pane)

    assertEquals(layoutInfo, state.layoutInfo)
  }

  @Test
  fun `opening another target on the same pane surface keeps layout info for visible area`() {
    val state = EditorSubPaneState()
    val rowPane = tableAxisPane(tableId = "table", index = 0)
    val colPane = tableAxisPane(tableId = "table", index = 1)
    state.open(rowPane)
    state.updateLayoutInfo(
      EditorSubPaneLayoutInfo(
        pane = rowPane,
        visibleHeight = 180f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      )
    )

    state.open(colPane)

    assertEquals(
      EditorSubPaneLayoutInfo(
        pane = colPane,
        visibleHeight = 180f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      ),
      state.layoutInfo,
    )
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
          pane = EditorSubPane.RelatedNotes,
          visibleHeight = 240f,
          visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
        )
      ),
    )
    assertEquals(
      0f,
      resolveSubPaneBottomOcclusion(
        EditorSubPaneLayoutInfo(
          pane = EditorSubPane.RelatedNotes,
          visibleHeight = 640f,
          visibleAreaMode = EditorSubPaneVisibleAreaMode.OverlayEditor,
        )
      ),
    )
  }

  @Test
  fun `stale layout info from another pane is ignored`() {
    val state = EditorSubPaneState()
    state.open(EditorSubPane.RelatedNotes)

    state.updateLayoutInfo(
      EditorSubPaneLayoutInfo(
        pane = tableAxisPane(tableId = "table"),
        visibleHeight = 240f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      )
    )

    assertNull(state.layoutInfo)
  }

  @Test
  fun `layout clear from another pane is ignored`() {
    val state = EditorSubPaneState()
    val layoutInfo =
      EditorSubPaneLayoutInfo(
        pane = EditorSubPane.RelatedNotes,
        visibleHeight = 240f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      )
    state.open(EditorSubPane.RelatedNotes)
    state.updateLayoutInfo(layoutInfo)

    state.clearLayoutInfo(tableAxisPane(tableId = "table"))

    assertEquals(layoutInfo, state.layoutInfo)
  }

  @Test
  fun `stale layout clear from previous table target does not clear active target`() {
    val state = EditorSubPaneState()
    val oldPane = tableAxisPane(tableId = "table", index = 0)
    val newPane = tableAxisPane(tableId = "table", index = 1)
    val layoutInfo =
      EditorSubPaneLayoutInfo(
        pane = newPane,
        visibleHeight = 180f,
        visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
      )
    state.open(oldPane)
    state.open(newPane)
    state.updateLayoutInfo(layoutInfo)

    state.clearLayoutInfo(oldPane)

    assertEquals(layoutInfo, state.layoutInfo)
  }

  private fun tableAxisPane(
    tableId: String,
    index: Int = 0,
    openedSelection: Selection? = null,
  ): EditorSubPane.TableAxisActions =
    EditorSubPane.TableAxisActions(
      target = tableAxisTarget(tableId = tableId, index = index),
      openedSelection = openedSelection,
    )

  private fun tableAxisTarget(tableId: String, index: Int = 0): EditorTableAxisActionsTarget =
    EditorTableAxisActionsTarget(
      tableId = tableId,
      axis = Axis.Horizontal,
      index = index,
      count = 3,
    )

  private fun selection(offset: Int): Selection {
    val position = Position(node = "text", offset = offset, affinity = Affinity.Downstream)
    return Selection(anchor = position, head = position)
  }
}
