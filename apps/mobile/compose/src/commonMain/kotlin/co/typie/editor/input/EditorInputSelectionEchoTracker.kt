package co.typie.editor.input

import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.ImeRange
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.SelectionOp

internal class EditorInputSelectionEchoTracker {
  private var nextToken = 0
  private val pending = mutableListOf<ExpectedSelectionEcho>()

  fun expect(
    direction: EditorInputSelectionEchoDirection,
    selection: ImeRange?,
    extend: Boolean,
  ): Int {
    val token = nextToken
    nextToken += 1
    pending +=
      ExpectedSelectionEcho(
        token = token,
        direction = direction,
        selection = selection,
        extend = extend,
      )
    return token
  }

  fun consumeIfEcho(messages: List<Message>): Boolean {
    val echo = pending.firstOrNull() ?: return false
    if (!echo.matches(messages)) return false
    pending.removeAt(0)
    return true
  }

  fun expire(token: Int) {
    pending.removeAll { it.token == token }
  }

  fun reset() {
    pending.clear()
  }

  private data class ExpectedSelectionEcho(
    val token: Int,
    val direction: EditorInputSelectionEchoDirection,
    val selection: ImeRange?,
    val extend: Boolean,
  ) {
    fun matches(messages: List<Message>): Boolean =
      matchesSelectionDelta(messages) || (extend && matchesExtendedSelection(messages))

    private fun matchesSelectionDelta(messages: List<Message>): Boolean {
      val directions = messages.mapNotNull { it.navigationDirectionOrNull() }
      if (directions.isEmpty() || directions.size != messages.size) return false
      return when (direction) {
        EditorInputSelectionEchoDirection.Backward -> directions.all { it == Direction.Backward }

        EditorInputSelectionEchoDirection.Forward -> directions.all { it == Direction.Forward }

        EditorInputSelectionEchoDirection.Vertical -> true
      }
    }

    private fun matchesExtendedSelection(messages: List<Message>): Boolean {
      val selection = selection ?: return false
      val op = messages.singleOrNull()?.selectionSetFlatOrNull() ?: return false
      val previousStart = minOf(selection.start, selection.end)
      val previousEnd = maxOf(selection.start, selection.end)
      val nextStart = minOf(op.start, op.end)
      val nextEnd = maxOf(op.start, op.end)
      return when (direction) {
        EditorInputSelectionEchoDirection.Backward ->
          nextStart < previousStart && nextEnd == previousEnd

        EditorInputSelectionEchoDirection.Forward ->
          nextStart == previousStart && nextEnd > previousEnd

        EditorInputSelectionEchoDirection.Vertical ->
          nextStart != nextEnd && (nextStart == previousStart || nextEnd == previousEnd)
      }
    }
  }
}

internal enum class EditorInputSelectionEchoDirection {
  Backward,
  Forward,
  Vertical,
}

private fun Message.navigationDirectionOrNull(): Direction? =
  when (this) {
    is Message.Navigation ->
      when (val op = op) {
        is NavigationOp.Move ->
          when (val movement = op.movement) {
            Movement.Grapheme(Direction.Backward) -> Direction.Backward
            Movement.Grapheme(Direction.Forward) -> Direction.Forward
            else -> null
          }
      }

    else -> null
  }

private fun Message.selectionSetFlatOrNull(): SelectionOp.SetFlat? =
  when (this) {
    is Message.Selection ->
      when (val op = op) {
        is SelectionOp.SetFlat -> op
        else -> null
      }

    else -> null
  }
