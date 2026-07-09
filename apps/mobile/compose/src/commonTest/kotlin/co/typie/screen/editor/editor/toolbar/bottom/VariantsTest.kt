package co.typie.screen.editor.editor.toolbar.bottom

import co.typie.editor.ffi.BlockquoteVariant
import co.typie.editor.ffi.HorizontalRuleVariant
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOp
import co.typie.screen.editor.editor.toolbar.BlockquoteVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.HorizontalRuleVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.TableBorderStylePanelTarget
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class VariantsTest {
  @Test
  fun current_horizontal_rule_variant_returns_no_message() {
    val target =
      HorizontalRuleVariantPanelTarget.Existing(
        nodeId = "hr",
        currentVariant = HorizontalRuleVariant.Line,
      )

    assertNull(target.messageOrNull(HorizontalRuleVariant.Line))
    assertEquals(
      Message.Node(
        NodeOp.SetAttrs(
          id = "hr",
          attrs = PlainNode.HorizontalRule(variant = HorizontalRuleVariant.DashedLine),
        )
      ),
      target.messageOrNull(HorizontalRuleVariant.DashedLine),
    )
  }

  @Test
  fun current_blockquote_variant_returns_no_message() {
    val target =
      BlockquoteVariantPanelTarget.Existing(
        nodeId = "blockquote",
        currentVariant = BlockquoteVariant.LeftLine,
      )

    assertNull(target.messageOrNull(BlockquoteVariant.LeftLine))
    assertEquals(
      Message.Node(
        NodeOp.SetAttrs(
          id = "blockquote",
          attrs = PlainNode.Blockquote(variant = BlockquoteVariant.LeftQuote),
        )
      ),
      target.messageOrNull(BlockquoteVariant.LeftQuote),
    )
  }

  @Test
  fun current_table_border_style_returns_no_message() {
    val target =
      TableBorderStylePanelTarget(tableId = "table", currentStyle = TableBorderStyle.Solid)

    assertNull(target.messageOrNull(TableBorderStyle.Solid))
    assertEquals(
      Message.Node(
        NodeOp.Table(
          id = "table",
          op = TableOp.SetBorderStyle(borderStyle = TableBorderStyle.Dashed),
        )
      ),
      target.messageOrNull(TableBorderStyle.Dashed),
    )
  }
}
