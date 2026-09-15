package co.typie.editor.input

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportTransform
import co.typie.editor.PagePoint
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.SelectionOp
import kotlin.math.abs

internal class EditorFloatingCursorSession
private constructor(
  private var origin: PagePoint,
  private var pendingEnd: PagePoint?,
  private val cursorAt: (PagePoint) -> CursorMetrics?,
) {
  fun update(dx: Float, dy: Float, transform: EditorViewportTransform): List<Message>? {
    val end = pendingEnd
    if (end != null) {
      if (dx == 0f && dy == 0f) return null
      // The delta only orders the candidates. Hit testing must confirm a caret move
      // in the matching direction before we collapse the selection.
      val backwardFirst = (if (abs(dx) > abs(dy)) dx else dy) < 0f
      for (backward in listOf(backwardFirst, !backwardFirst)) {
        val candidateOrigin = if (backward) end else origin
        val point = resolveFloatingCursorPoint(candidateOrigin, dx, dy, transform) ?: continue
        // A handle can lie in the blank area after a paragraph break. Compare
        // hit-tested carets so the handle/caret gap is not mistaken for movement.
        val initial = cursorAt(candidateOrigin)?.lineCenter() ?: continue
        val hit = cursorAt(point)?.lineCenter() ?: continue
        val movement = compareValuesBy(hit, initial, { it.page }, { it.y }, { it.x })
        if (if (backward) movement >= 0 else movement <= 0) continue

        origin = candidateOrigin
        pendingEnd = null
        return point.toSelectionMessages()
      }
      return null
    }
    val point =
      resolveFloatingCursorPoint(origin = origin, dx = dx, dy = dy, transform = transform)
        ?: return null
    return point.toSelectionMessages()
  }

  companion object {
    fun begin(
      cursor: CursorMetrics?,
      selectionEndpoints: SelectionEndpoints?,
      cursorAt: (PagePoint) -> CursorMetrics?,
    ): EditorFloatingCursorSession? {
      if (cursor != null) {
        return EditorFloatingCursorSession(
          origin = PagePoint(cursor.pageIdx, cursor.caret.x, cursor.line.y),
          pendingEnd = null,
          cursorAt = cursorAt,
        )
      }
      val endpoints = selectionEndpoints ?: return null
      return EditorFloatingCursorSession(
        endpoints.from.lineCenter(),
        endpoints.to.lineCenter(),
        cursorAt,
      )
    }
  }
}

private fun PageRect.lineCenter(): PagePoint = PagePoint(pageIdx, rect.x, rect.y + rect.height / 2f)

private fun CursorMetrics.lineCenter(): PagePoint =
  PagePoint(pageIdx, caret.x, line.y + line.height / 2f)

internal fun resolveFloatingCursorPoint(
  origin: PagePoint,
  dx: Float,
  dy: Float,
  transform: EditorViewportTransform,
): PagePoint? {
  val originGlobal =
    transform.localToGlobal(page = origin.page, x = origin.x, y = origin.y) ?: return null
  val targetGlobal = originGlobal + Offset(dx, dy)
  return transform.globalToLocal(x = targetGlobal.x, y = targetGlobal.y)
}

private fun PagePoint.toSelectionMessages(): List<Message> =
  listOf(Message.Selection(SelectionOp.SetAt(page = page, x = x, y = y)))
