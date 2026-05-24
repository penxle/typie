package co.typie.editor

import androidx.compose.ui.input.key.Key as ComposeKey
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Break
import co.typie.editor.ffi.ClipboardOp
import co.typie.editor.ffi.DeletionOp
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.HistoryOp
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.platform.Clipboard
import co.typie.platform.Platform

internal enum class KeyModifier {
  Shift,
  Mod,
  Ctrl,
  Alt,
}

internal data class KeyBinding(
  val key: ComposeKey,
  val modifiers: Set<KeyModifier> = emptySet(),
  val predicate: (() -> Boolean)? = null,
  // TODO(editor-parity): movement/selection shortcut은 키별 고정 target보다, dispatch 이후의
  // 실제 scroll anchor(selection head 또는 cursor)를 따라가도록 정리해야 한다.
  val bringIntoViewTarget: EditorBringIntoViewTarget? = EditorBringIntoViewTarget.CurrentCursorLine,
  val action: suspend Editor.(Clipboard) -> List<Message>,
)

private fun move(movement: Movement, extend: Boolean): Message =
  Message.Navigation(NavigationOp.Move(movement, extend))

private fun toggleModifier(type: ModifierType): Message = Message.Modifier(ModifierOp.Toggle(type))

private fun delete(movement: Movement): Message = Message.Deletion(DeletionOp.Move(movement))

internal fun createBindings(platform: Platform): List<KeyBinding> {
  val isMac = platform != Platform.Android

  return listOf(
    KeyBinding(
      ComposeKey.DirectionLeft,
      action = { listOf(move(Movement.Grapheme(Direction.Backward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionLeft,
      setOf(KeyModifier.Shift),
      action = { listOf(move(Movement.Grapheme(Direction.Backward), true)) },
    ),
    KeyBinding(
      ComposeKey.DirectionRight,
      action = { listOf(move(Movement.Grapheme(Direction.Forward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionRight,
      setOf(KeyModifier.Shift),
      action = { listOf(move(Movement.Grapheme(Direction.Forward), true)) },
    ),
    KeyBinding(
      ComposeKey.DirectionLeft,
      setOf(KeyModifier.Alt),
      action = { listOf(move(Movement.Word(Direction.Backward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionRight,
      setOf(KeyModifier.Alt),
      action = { listOf(move(Movement.Word(Direction.Forward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionLeft,
      setOf(KeyModifier.Alt, KeyModifier.Shift),
      action = { listOf(move(Movement.Word(Direction.Backward), true)) },
    ),
    KeyBinding(
      ComposeKey.DirectionRight,
      setOf(KeyModifier.Alt, KeyModifier.Shift),
      action = { listOf(move(Movement.Word(Direction.Forward), true)) },
    ),
    KeyBinding(
      ComposeKey.DirectionLeft,
      setOf(KeyModifier.Ctrl),
      action = { listOf(move(Movement.Word(Direction.Backward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionRight,
      setOf(KeyModifier.Ctrl),
      action = { listOf(move(Movement.Word(Direction.Forward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionLeft,
      setOf(KeyModifier.Mod),
      predicate = { isMac },
      action = { listOf(move(Movement.Line(Direction.Backward, Axis.Horizontal), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionRight,
      setOf(KeyModifier.Mod),
      predicate = { isMac },
      action = { listOf(move(Movement.Line(Direction.Forward, Axis.Horizontal), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionLeft,
      setOf(KeyModifier.Mod, KeyModifier.Shift),
      predicate = { isMac },
      action = { listOf(move(Movement.Line(Direction.Backward, Axis.Horizontal), true)) },
    ),
    KeyBinding(
      ComposeKey.DirectionRight,
      setOf(KeyModifier.Mod, KeyModifier.Shift),
      predicate = { isMac },
      action = { listOf(move(Movement.Line(Direction.Forward, Axis.Horizontal), true)) },
    ),
    KeyBinding(
      ComposeKey.DirectionUp,
      action = { listOf(move(Movement.Line(Direction.Backward, Axis.Vertical), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionUp,
      setOf(KeyModifier.Shift),
      action = { listOf(move(Movement.Line(Direction.Backward, Axis.Vertical), true)) },
    ),
    KeyBinding(
      ComposeKey.DirectionDown,
      action = { listOf(move(Movement.Line(Direction.Forward, Axis.Vertical), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionDown,
      setOf(KeyModifier.Shift),
      action = { listOf(move(Movement.Line(Direction.Forward, Axis.Vertical), true)) },
    ),
    KeyBinding(
      ComposeKey.DirectionUp,
      setOf(KeyModifier.Mod),
      predicate = { isMac },
      action = { listOf(move(Movement.Document(Direction.Backward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionDown,
      setOf(KeyModifier.Mod),
      predicate = { isMac },
      action = { listOf(move(Movement.Document(Direction.Forward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionUp,
      setOf(KeyModifier.Alt),
      action = { listOf(move(Movement.Sentence(Direction.Backward), false)) },
    ),
    KeyBinding(
      ComposeKey.DirectionDown,
      setOf(KeyModifier.Alt),
      action = { listOf(move(Movement.Sentence(Direction.Forward), false)) },
    ),
    KeyBinding(ComposeKey.Enter, action = { listOf(Message.Key(FfiKeyEvent(FfiKey.Enter))) }),
    KeyBinding(
      ComposeKey.Enter,
      setOf(KeyModifier.Shift),
      action = { listOf(Message.Insertion(InsertionOp.Break(Break.Line))) },
    ),
    KeyBinding(
      ComposeKey.Enter,
      setOf(KeyModifier.Mod),
      action = { listOf(Message.Insertion(InsertionOp.Break(Break.Page))) },
    ),
    KeyBinding(
      ComposeKey.Backspace,
      action = { listOf(Message.Key(FfiKeyEvent(FfiKey.Backspace))) },
    ),
    KeyBinding(
      ComposeKey.Backspace,
      setOf(KeyModifier.Alt),
      action = { listOf(delete(Movement.Word(Direction.Backward))) },
    ),
    KeyBinding(
      ComposeKey.Backspace,
      setOf(KeyModifier.Ctrl),
      action = { listOf(delete(Movement.Word(Direction.Backward))) },
    ),
    KeyBinding(
      ComposeKey.Backspace,
      setOf(KeyModifier.Mod),
      predicate = { isMac },
      action = { listOf(delete(Movement.Line(Direction.Backward, Axis.Horizontal))) },
    ),
    KeyBinding(ComposeKey.Delete, action = { listOf(Message.Key(FfiKeyEvent(FfiKey.Delete))) }),
    KeyBinding(
      ComposeKey.Delete,
      setOf(KeyModifier.Alt),
      action = { listOf(delete(Movement.Word(Direction.Forward))) },
    ),
    KeyBinding(ComposeKey.Tab, action = { listOf(Message.Key(FfiKeyEvent(FfiKey.Tab))) }),
    KeyBinding(
      ComposeKey.Escape,
      bringIntoViewTarget = null,
      action = { listOf(Message.Key(FfiKeyEvent(FfiKey.Escape))) },
    ),
    KeyBinding(
      ComposeKey.MoveHome,
      action = { listOf(move(Movement.Line(Direction.Backward, Axis.Horizontal), false)) },
    ),
    KeyBinding(
      ComposeKey.MoveHome,
      setOf(KeyModifier.Ctrl),
      action = { listOf(move(Movement.Document(Direction.Backward), false)) },
    ),
    KeyBinding(
      ComposeKey.MoveEnd,
      action = { listOf(move(Movement.Line(Direction.Forward, Axis.Horizontal), false)) },
    ),
    KeyBinding(
      ComposeKey.MoveEnd,
      setOf(KeyModifier.Ctrl),
      action = { listOf(move(Movement.Document(Direction.Forward), false)) },
    ),
    KeyBinding(
      ComposeKey.PageUp,
      action = { listOf(move(Movement.Page(Direction.Backward), false)) },
    ),
    KeyBinding(
      ComposeKey.PageDown,
      action = { listOf(move(Movement.Page(Direction.Forward), false)) },
    ),
    KeyBinding(
      ComposeKey.B,
      setOf(KeyModifier.Mod),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { listOf(toggleModifier(ModifierType.Bold)) },
    ),
    KeyBinding(
      ComposeKey.I,
      setOf(KeyModifier.Mod),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { listOf(toggleModifier(ModifierType.Italic)) },
    ),
    KeyBinding(
      ComposeKey.S,
      setOf(KeyModifier.Mod, KeyModifier.Shift),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { listOf(toggleModifier(ModifierType.Strikethrough)) },
    ),
    KeyBinding(
      ComposeKey.U,
      setOf(KeyModifier.Mod),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { listOf(toggleModifier(ModifierType.Underline)) },
    ),
    KeyBinding(
      ComposeKey.Backslash,
      setOf(KeyModifier.Mod),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { listOf(Message.Modifier(ModifierOp.ClearAll)) },
    ),
    KeyBinding(
      ComposeKey.Z,
      setOf(KeyModifier.Mod),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { listOf(Message.History(HistoryOp.Undo)) },
    ),
    KeyBinding(
      ComposeKey.Z,
      setOf(KeyModifier.Mod, KeyModifier.Shift),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { listOf(Message.History(HistoryOp.Redo)) },
    ),
    KeyBinding(
      ComposeKey.C,
      setOf(KeyModifier.Mod),
      bringIntoViewTarget = null,
      action = { clipboard ->
        copySelection()?.let { clipboard.copyRichText(html = it.html, text = it.text) }
        emptyList()
      },
    ),
    KeyBinding(
      ComposeKey.X,
      setOf(KeyModifier.Mod),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { clipboard ->
        val payload = copySelection() ?: return@KeyBinding emptyList()
        if (clipboard.copyRichText(html = payload.html, text = payload.text)) {
          listOf(Message.Clipboard(ClipboardOp.Cut))
        } else {
          emptyList()
        }
      },
    ),
    KeyBinding(
      ComposeKey.V,
      setOf(KeyModifier.Mod),
      action = { clipboard ->
        val read = clipboard.paste() ?: return@KeyBinding emptyList()
        listOf(Message.Clipboard(ClipboardOp.Paste(html = read.html, text = read.text)))
      },
    ),
    KeyBinding(
      ComposeKey.A,
      setOf(KeyModifier.Mod),
      bringIntoViewTarget = EditorBringIntoViewTarget.CurrentSelectionHead,
      action = { listOf(Message.Selection(SelectionOp.Expand(SelectionExpansionUnit.All))) },
    ),
    KeyBinding(
      ComposeKey.Q,
      setOf(KeyModifier.Ctrl),
      predicate = { isMac },
      bringIntoViewTarget = null,
    ) {
      inspectState()
      emptyList()
    },
    KeyBinding(
      ComposeKey.W,
      setOf(KeyModifier.Ctrl),
      predicate = { isMac },
      bringIntoViewTarget = null,
    ) {
      inspectStateAsMacro()
      emptyList()
    },
  )
}

internal fun matchesKeyBinding(binding: KeyBinding, platform: Platform, event: KeyEvent): Boolean {
  if (binding.key != event.key) return false

  val mods = binding.modifiers
  val isMac = platform != Platform.Android

  val expectShift = KeyModifier.Shift in mods
  val expectAlt = KeyModifier.Alt in mods
  val expectCtrl = KeyModifier.Ctrl in mods || (!isMac && KeyModifier.Mod in mods)
  val expectMeta = isMac && KeyModifier.Mod in mods

  if (event.isShiftPressed != expectShift) return false
  if (event.isAltPressed != expectAlt) return false
  if (event.isCtrlPressed != expectCtrl) return false
  if (event.isMetaPressed != expectMeta) return false

  if (binding.predicate != null && !binding.predicate.invoke()) return false

  return true
}
