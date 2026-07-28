package co.typie.screen.editor.editor

import androidx.compose.foundation.focusable
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.onPreviewKeyEvent
import co.typie.editor.ffi.Selection
import co.typie.ui.utils.ShortcutModifier
import co.typie.ui.utils.matchesShortcut

internal data class EditorScreenShortcutContext(
  val sceneInForeground: Boolean,
  val subPaneBlocksEditorInput: Boolean,
  val editorFocused: Boolean,
  val findReplaceActive: Boolean,
  val spellcheckActive: Boolean,
  val aiFeedbackActive: Boolean,
)

internal data class EditorScreenShortcutActions(
  val openFindReplace: () -> Unit,
  val closeFindReplace: () -> Unit,
  val closeSpellcheck: () -> Unit,
  val closeAiFeedback: () -> Unit,
)

private data class EditorScreenShortcutBinding(
  val key: Key,
  val modifiers: Set<ShortcutModifier> = emptySet(),
  val enabled: (EditorScreenShortcutContext) -> Boolean = { true },
  val action: (EditorScreenShortcutContext, EditorScreenShortcutActions) -> Boolean,
)

private val EditorScreenShortcutBindings =
  listOf(
    EditorScreenShortcutBinding(
      key = Key.F,
      modifiers = setOf(ShortcutModifier.Mod),
      enabled = ::isEditorScreenShortcutAvailable,
      action = { _, actions ->
        actions.openFindReplace()
        true
      },
    ),
    EditorScreenShortcutBinding(
      key = Key.Escape,
      enabled = ::isEditorScreenShortcutAvailable,
      action = ::handleEscapeShortcut,
    ),
  )

internal fun handleEditorScreenShortcut(
  event: KeyEvent,
  context: EditorScreenShortcutContext,
  actions: EditorScreenShortcutActions,
): Boolean {
  val binding =
    EditorScreenShortcutBindings.firstOrNull { binding ->
      binding.enabled(context) &&
        matchesShortcut(event = event, key = binding.key, modifiers = binding.modifiers)
    } ?: return false

  return binding.action(context, actions)
}

@Composable
internal fun Modifier.editorScreenShortcutFocusTarget(
  active: Boolean,
  enabled: Boolean,
  editorFocused: Boolean,
  selection: Selection?,
  onPreviewKeyEvent: (KeyEvent) -> Boolean,
): Modifier {
  val focusRequester = remember { FocusRequester() }
  var previousSelection by remember { mutableStateOf(selection) }
  var fallbackFocusPending by remember { mutableStateOf(false) }

  LaunchedEffect(active, enabled, editorFocused, selection) {
    val selectionCleared = previousSelection != null && selection == null
    previousSelection = selection

    if (!active || !enabled || selection != null) {
      fallbackFocusPending = false
      return@LaunchedEffect
    }

    if (selectionCleared) {
      fallbackFocusPending = true
    }

    if (fallbackFocusPending && !editorFocused) {
      // After Escape clears the editor selection, no text input owns focus. Focus this non-text
      // target on the next frame so a following Escape can close the active editor mode.
      withFrameNanos {}
      focusRequester.requestFocus()
      fallbackFocusPending = false
    }
  }

  return focusRequester(focusRequester)
    .onPreviewKeyEvent(onPreviewKeyEvent)
    .focusable(enabled = active && enabled)
}

private fun isEditorScreenShortcutAvailable(context: EditorScreenShortcutContext): Boolean =
  context.sceneInForeground && !context.subPaneBlocksEditorInput

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
