package co.typie.screen.editor.editor

import androidx.compose.runtime.Composable
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import co.typie.ui.input.WindowInputHandler
import co.typie.ui.utils.ShortcutModifier
import co.typie.ui.utils.matchesShortcut

internal data class EditorScreenShortcutContext(
  val enabled: Boolean,
  val editorFocused: Boolean,
  val findReplaceActive: Boolean,
  val spellcheckActive: Boolean,
  val aiFeedbackActive: Boolean,
)

internal data class EditorScreenShortcutActions(
  val openFindReplace: () -> Unit,
  val resetZoom: () -> Unit,
  val zoomIn: () -> Unit,
  val zoomOut: () -> Unit,
  val closeFindReplace: () -> Unit,
  val closeSpellcheck: () -> Unit,
  val closeAiFeedback: () -> Unit,
)

private data class EditorScreenShortcutBinding(
  val key: Key,
  val modifiers: Set<ShortcutModifier> = emptySet(),
  val action: (EditorScreenShortcutContext, EditorScreenShortcutActions) -> Boolean,
)

private val EditorScreenShortcutBindings =
  listOf(Key.Equals, Key.Plus, Key.NumPadAdd).flatMap { key ->
    listOf(setOf(ShortcutModifier.Mod), setOf(ShortcutModifier.Mod, ShortcutModifier.Shift)).map {
      modifiers ->
      EditorScreenShortcutBinding(key, modifiers) { _, actions ->
        actions.zoomIn()
        true
      }
    }
  } +
    listOf(Key.Minus, Key.NumPadSubtract).map { key ->
      EditorScreenShortcutBinding(key, setOf(ShortcutModifier.Mod)) { _, actions ->
        actions.zoomOut()
        true
      }
    } +
    listOf(
      EditorScreenShortcutBinding(
        key = Key.Zero,
        modifiers = setOf(ShortcutModifier.Mod),
        action = { _, actions ->
          actions.resetZoom()
          true
        },
      ),
      EditorScreenShortcutBinding(
        key = Key.NumPad0,
        modifiers = setOf(ShortcutModifier.Mod),
        action = { _, actions ->
          actions.resetZoom()
          true
        },
      ),
      EditorScreenShortcutBinding(
        key = Key.F,
        modifiers = setOf(ShortcutModifier.Mod),
        action = { _, actions ->
          actions.openFindReplace()
          true
        },
      ),
      EditorScreenShortcutBinding(key = Key.Escape, action = ::handleEscapeShortcut),
    )

internal fun handleEditorScreenShortcut(
  event: KeyEvent,
  context: EditorScreenShortcutContext,
  actions: EditorScreenShortcutActions,
): Boolean {
  if (!context.enabled) return false
  val binding =
    EditorScreenShortcutBindings.firstOrNull { binding ->
      matchesShortcut(event = event, key = binding.key, modifiers = binding.modifiers)
    } ?: return false

  return binding.action(context, actions)
}

/**
 * Zoom/find belong to the document screen, including title, search and subpane focus. Text editing
 * and the focused editor's Escape/IME handling remain with editor input.
 */
@Composable
internal fun EditorScreenShortcuts(
  context: EditorScreenShortcutContext,
  actions: EditorScreenShortcutActions,
) {
  WindowInputHandler(
    enabled = context.enabled,
    onKeyEvent = { handleEditorScreenShortcut(it, context, actions) },
  )
}

private fun handleEscapeShortcut(
  context: EditorScreenShortcutContext,
  actions: EditorScreenShortcutActions,
): Boolean {
  if (context.editorFocused) {
    return false
  }

  when {
    context.findReplaceActive -> actions.closeFindReplace()
    context.spellcheckActive -> actions.closeSpellcheck()
    context.aiFeedbackActive -> actions.closeAiFeedback()
    else -> return false
  }

  return true
}
