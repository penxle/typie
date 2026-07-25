package co.typie.screen.editor.editor

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import co.typie.screen.editor.editor.state.EditorInputEffect

@Stable
internal class EditorScreenEditingState {
  var editing by mutableStateOf(false)
    private set

  private var nextReadingTransition = 0L
  private var pendingReadingTransition: Long? = null

  fun enterEditing() {
    pendingReadingTransition = null
    editing = true
  }

  fun enterReading() {
    pendingReadingTransition = null
    editing = false
  }

  fun beginReadingTransition(): Long? {
    if (!editing || pendingReadingTransition != null) {
      return null
    }
    val transition = ++nextReadingTransition
    pendingReadingTransition = transition
    return transition
  }

  fun isReadingTransitionCurrent(transition: Long): Boolean =
    editing && pendingReadingTransition == transition

  fun completeReadingTransition(transition: Long): Boolean {
    if (!isReadingTransitionCurrent(transition)) {
      return false
    }
    pendingReadingTransition = null
    editing = false
    return true
  }

  fun cancelReadingTransition(transition: Long) {
    if (pendingReadingTransition == transition) {
      pendingReadingTransition = null
    }
  }

  fun directEditingEnabled(readOnly: Boolean): Boolean = editing && !readOnly

  fun shouldRunReadingCleanup(readOnly: Boolean): Boolean = !directEditingEnabled(readOnly)
}

@Composable
internal fun rememberEditorScreenEditingState(entityId: String): EditorScreenEditingState =
  remember(entityId) { EditorScreenEditingState() }

internal fun performEditorInputEffects(
  effects: List<EditorInputEffect>,
  showKeyboard: () -> Unit,
  hideKeyboard: () -> Unit,
  requestFocus: () -> Unit,
  clearFocus: () -> Unit,
  enterReadingMode: () -> Unit,
) {
  effects.forEach { effect ->
    when (effect) {
      EditorInputEffect.ShowKeyboard -> showKeyboard()
      EditorInputEffect.HideKeyboard -> hideKeyboard()
      EditorInputEffect.RequestFocus -> requestFocus()
      EditorInputEffect.ClearFocus -> clearFocus()
      EditorInputEffect.EnterReadingMode -> enterReadingMode()
    }
  }
}
