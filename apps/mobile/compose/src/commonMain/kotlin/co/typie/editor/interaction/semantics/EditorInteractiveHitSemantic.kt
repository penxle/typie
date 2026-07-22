package co.typie.editor.interaction.semantics

import co.typie.editor.Editor
import co.typie.editor.PagePoint
import co.typie.editor.ffi.InteractiveHit
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.ViewOp

internal class EditorInteractiveHitSemantic {
  fun handleTap(editor: Editor, point: PagePoint): Boolean =
    when (val hit = editor.interactiveHitTest(page = point.page, x = point.x, y = point.y)) {
      is InteractiveHit.FoldTitle -> {
        val onText = hit.textRect?.contains(point.x, point.y) == true
        if (onText) {
          false
        } else {
          editor.enqueue(Message.View(ViewOp.ToggleFold(id = hit.id)))
          true
        }
      }
      is InteractiveHit.CalloutIcon -> {
        editor.enqueue(
          Message.Node(
            NodeOp.SetAttrs(id = hit.id, attrs = PlainNode.Callout(variant = hit.nextVariant))
          )
        )
        true
      }
      else -> false
    }
}

private fun Rect.contains(x: Float, y: Float): Boolean =
  x >= this.x && x <= this.x + width && y >= this.y && y <= this.y + height
