package co.typie.editor

import androidx.compose.ui.input.key.Key as ComposeKey
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.Direction
import co.typie.editor.ffi.Key as FfiKey
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.ModifierOp
import co.typie.editor.ffi.ModifierType
import co.typie.editor.ffi.Movement
import co.typie.editor.ffi.NavigationOp
import co.typie.platform.Platform
import co.typie.screen.editor.editor.scroll.EditorScrollController
import co.typie.screen.editor.editor.scroll.EditorScrollTarget
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

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
  val scrollTarget: EditorScrollTarget? = EditorScrollTarget.CurrentCursor,
  val action: Editor.() -> List<Message>,
)

private fun move(movement: Movement, extend: Boolean): Message =
  Message.Navigation(NavigationOp.Move(movement, extend))

private fun toggleModifier(type: ModifierType): Message = Message.Modifier(ModifierOp.Toggle(type))

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
    KeyBinding(ComposeKey.Enter, action = { listOf(Message.Key(FfiKeyEvent(FfiKey.Enter))) }),
    KeyBinding(
      ComposeKey.Backspace,
      action = { listOf(Message.Key(FfiKeyEvent(FfiKey.Backspace))) },
    ),
    KeyBinding(
      ComposeKey.B,
      setOf(KeyModifier.Mod),
      scrollTarget = EditorScrollTarget.CurrentSelectionHead,
      action = { listOf(toggleModifier(ModifierType.Bold)) },
    ),
    KeyBinding(
      ComposeKey.I,
      setOf(KeyModifier.Mod),
      scrollTarget = EditorScrollTarget.CurrentSelectionHead,
      action = { listOf(toggleModifier(ModifierType.Italic)) },
    ),
    KeyBinding(
      ComposeKey.S,
      setOf(KeyModifier.Mod, KeyModifier.Shift),
      scrollTarget = EditorScrollTarget.CurrentSelectionHead,
      action = { listOf(toggleModifier(ModifierType.Strikethrough)) },
    ),
    KeyBinding(
      ComposeKey.U,
      setOf(KeyModifier.Mod, KeyModifier.Shift),
      scrollTarget = EditorScrollTarget.CurrentSelectionHead,
      action = { listOf(toggleModifier(ModifierType.Underline)) },
    ),
    KeyBinding(ComposeKey.Q, setOf(KeyModifier.Ctrl), predicate = { isMac }, scrollTarget = null) {
      inspectState()
      emptyList()
    },
    KeyBinding(ComposeKey.W, setOf(KeyModifier.Ctrl), predicate = { isMac }, scrollTarget = null) {
      inspectStateAsMacro()
      emptyList()
    },
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

internal fun handleKeyDown(
  editor: Editor,
  platform: Platform,
  bindings: List<KeyBinding>,
  scrollController: EditorScrollController?,
  coroutineScope: CoroutineScope,
  event: KeyEvent,
): Boolean {
  val binding = bindings.find { matchBinding(it, platform, event) } ?: return false
  val messages = binding.action(editor)
  if (messages.isNotEmpty()) {
    coroutineScope.launch {
      editor.dispatch(*messages.toTypedArray())
      binding.scrollTarget?.let { scrollController?.request(target = it) }
    }
  }
  return true
}
