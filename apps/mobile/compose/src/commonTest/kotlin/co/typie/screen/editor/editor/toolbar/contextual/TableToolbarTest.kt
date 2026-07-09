package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Modifier as EditorModifier
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.TableOp
import co.typie.screen.editor.editor.toolbar.EditorToolbarTableTarget
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class TableToolbarTest {
  @Test
  fun table_select_all_uses_select_axis_without_axis_or_index() {
    assertEquals(
      Message.Node(NodeOp.Table(id = "table", op = TableOp.SelectAxis(axis = null, index = null))),
      tableSelectAllMessage("table"),
    )
  }

  @Test
  fun current_table_alignment_returns_no_message() {
    val target = tableTarget(align = Alignment.Left)

    assertNull(tableAlignmentMessageOrNull(target, Alignment.Left))
    assertEquals(
      Message.Modifier(
        ModifierOp.SetOnNode(id = "table", modifier = EditorModifier.Alignment(Alignment.Center))
      ),
      tableAlignmentMessageOrNull(target, Alignment.Center),
    )
  }

  @Test
  fun table_cell_background_uses_cell_background_op() {
    assertEquals(
      Message.Node(
        NodeOp.Table(id = "table", op = TableOp.SetCellBackgroundColor(color = "yellow"))
      ),
      tableCellBackgroundMessage("table", "yellow"),
    )
    assertEquals(
      Message.Node(NodeOp.Table(id = "table", op = TableOp.SetCellBackgroundColor(color = null))),
      tableCellBackgroundMessage("table", "none"),
    )
  }

  private fun tableTarget(align: Alignment): EditorToolbarTableTarget =
    EditorToolbarTableTarget(
      id = "table",
      node = PlainNode.Table(),
      selected = false,
      align = align,
      cellBackgroundCurrentValue = null,
    )
}
