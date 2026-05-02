package co.typie.editor.input

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportTransform
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PointerEvent

internal class EditorFloatingCursorSession {
  private var origin: EditorFloatingCursorOrigin? = null

  fun begin(cursor: CursorMetrics?) {
    origin = cursor?.let {
      EditorFloatingCursorOrigin(page = it.pageIdx, x = it.caret.x, y = it.line.y)
    }
  }

  fun update(dx: Float, dy: Float, transform: EditorViewportTransform): List<Message>? {
    val origin = origin ?: return null
    val point =
      resolveFloatingCursorPoint(origin = origin, dx = dx, dy = dy, transform = transform)
        ?: return null
    return point.toPointerClickMessages()
  }

  fun end() {
    origin = null
  }
}

internal data class EditorFloatingCursorOrigin(val page: Int, val x: Float, val y: Float)

internal fun resolveFloatingCursorPoint(
  origin: EditorFloatingCursorOrigin,
  dx: Float,
  dy: Float,
  transform: EditorViewportTransform,
): PagePoint? {
  val originGlobal =
    transform.localToGlobal(page = origin.page, x = origin.x, y = origin.y) ?: return null
  val targetGlobal = originGlobal + Offset(dx, dy)
  return transform.globalToLocal(x = targetGlobal.x, y = targetGlobal.y)
}

private fun PagePoint.toPointerClickMessages(): List<Message> =
  listOf(
    Message.Pointer(PointerEvent.Down(page = page, x = x, y = y, count = 1)),
    Message.Pointer(PointerEvent.Up),
  )
