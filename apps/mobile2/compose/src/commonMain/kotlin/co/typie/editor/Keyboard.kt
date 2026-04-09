package co.typie.editor

import androidx.compose.ui.input.key.Key as ComposeKey
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import co.typie.di.Platform
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.FormattingOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent

internal enum class KeyModifier { Shift, Mod, Ctrl, Alt }

internal data class KeyBinding(
  val key: ComposeKey,
  val modifiers: Set<KeyModifier> = emptySet(),
  val predicate: (() -> Boolean)? = null,
  val action: (Editor) -> Unit,
)

private fun move(movement: Movement, extend: Boolean): Message =
  Message.Navigation(NavigationOp.Move(movement, extend))

private fun toggleModifier(type: ModifierType): Message =
  Message.Formatting(FormattingOp.ToggleModifier(type))

internal fun createBindings(platform: Platform): List<KeyBinding> {
  val isMac = platform != Platform.Android

  return listOf(
    KeyBinding(ComposeKey.DirectionLeft) { it.enqueue(move(Movement.Grapheme(Direction.Backward), false)) },
    KeyBinding(ComposeKey.DirectionLeft, setOf(KeyModifier.Shift)) { it.enqueue(move(Movement.Grapheme(Direction.Backward), true)) },
    KeyBinding(ComposeKey.DirectionRight) { it.enqueue(move(Movement.Grapheme(Direction.Forward), false)) },
    KeyBinding(ComposeKey.DirectionRight, setOf(KeyModifier.Shift)) { it.enqueue(move(Movement.Grapheme(Direction.Forward), true)) },

    KeyBinding(ComposeKey.DirectionUp) { it.enqueue(move(Movement.Line(Direction.Backward, Axis.Vertical), false)) },
    KeyBinding(ComposeKey.DirectionUp, setOf(KeyModifier.Shift)) { it.enqueue(move(Movement.Line(Direction.Backward, Axis.Vertical), true)) },
    KeyBinding(ComposeKey.DirectionDown) { it.enqueue(move(Movement.Line(Direction.Forward, Axis.Vertical), false)) },
    KeyBinding(ComposeKey.DirectionDown, setOf(KeyModifier.Shift)) { it.enqueue(move(Movement.Line(Direction.Forward, Axis.Vertical), true)) },

    KeyBinding(ComposeKey.Enter) { it.enqueue(Message.Key(FfiKeyEvent(FfiKey.Enter))) },
    KeyBinding(ComposeKey.Backspace) { it.enqueue(Message.Key(FfiKeyEvent(FfiKey.Backspace))) },

    KeyBinding(ComposeKey.B, setOf(KeyModifier.Mod)) { it.enqueue(toggleModifier(ModifierType.Bold)) },
    KeyBinding(ComposeKey.I, setOf(KeyModifier.Mod)) { it.enqueue(toggleModifier(ModifierType.Italic)) },
    KeyBinding(ComposeKey.S, setOf(KeyModifier.Mod, KeyModifier.Shift)) { it.enqueue(toggleModifier(ModifierType.Strikethrough)) },
    KeyBinding(ComposeKey.U, setOf(KeyModifier.Mod, KeyModifier.Shift)) { it.enqueue(toggleModifier(ModifierType.Underline)) },

    KeyBinding(ComposeKey.Q, setOf(KeyModifier.Ctrl), predicate = { isMac }) { it.inspectState() },
    KeyBinding(ComposeKey.W, setOf(KeyModifier.Ctrl), predicate = { isMac }) { it.inspectStateAsMacro() },
  )
}

private fun matchBinding(binding: KeyBinding, platform: Platform, event: KeyEvent): Boolean {
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

internal fun handleKeyDown(editor: Editor, platform: Platform, bindings: List<KeyBinding>, event: KeyEvent): Boolean {
  val binding = bindings.find { matchBinding(it, platform, event) } ?: return false
  binding.action(editor)
  return true
}
