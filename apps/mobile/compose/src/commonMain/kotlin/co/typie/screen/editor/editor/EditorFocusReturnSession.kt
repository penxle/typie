package co.typie.screen.editor.editor

import androidx.compose.runtime.Stable
import androidx.compose.runtime.withFrameNanos
import co.typie.editor.Editor
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.StableSelection
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.async
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.ensureActive

@Stable
internal class EditorFocusReturnSession(
  private val scope: CoroutineScope,
  private val freezeSelection: suspend (Editor, Selection) -> StableSelection? =
    { editor, selection ->
      editor.freezeSelection(selection)
    },
  private val applySelection: suspend (Editor, StableSelection) -> Unit = { editor, selection ->
    editor.sync { enqueue(Message.Selection(SelectionOp.SetFrozen(selection = selection))) }
  },
  private val focusEditor: (Editor) -> Unit = { it.focus() },
  private val awaitFocusBoundary: suspend () -> Unit = { withFrameNanos {} },
) {
  private var currentContext: EditorContext? = null
  private var state: State = State.Idle

  fun observeEditorContext(
    editor: Editor?,
    focused: Boolean,
    selection: Selection?,
    contextActive: Boolean,
    auxiliaryOwnerActive: Boolean,
  ) {
    if (!contextActive || editor == null) {
      if (currentContext != null || state !is State.Idle) {
        invalidate()
      }
      return
    }

    val context =
      currentContext?.takeIf { it.editor === editor }
        ?: run {
          clearState(resetContext = true)
          EditorContext(editor).also { currentContext = it }
        }

    when (val current = state) {
      State.Idle -> {
        if (focused && selection != null) beginEligible(context, selection)
      }
      is State.Eligible -> {
        when {
          focused && selection != null -> beginEligible(context, selection)
          focused && selection == null -> clearState(resetContext = false)
          auxiliaryOwnerActive -> capture(current)
          else -> clearState(resetContext = false)
        }
      }
      is State.Captured -> Unit
    }
  }

  private fun capture(eligible: State.Eligible) {
    if (!isRestorable(eligible.context)) return
    state =
      State.Captured(
        context = eligible.context,
        stableSelection =
          scope.async {
            try {
              freezeSelection(eligible.context.editor, eligible.selection)
            } catch (error: CancellationException) {
              throw error
            } catch (_: Throwable) {
              null
            }
          },
      )
  }

  suspend fun restore() {
    val captured = state as? State.Captured ?: return
    awaitFocusBoundary()
    if (state !== captured) return

    state = State.Idle
    if (!isRestorable(captured.context)) {
      captured.stableSelection.cancel()
      return
    }

    if (captured.context.editor.currentSelection() != null) {
      captured.stableSelection.cancel()
      focusBestEffort(captured.context)
      return
    }

    val stableSelection = captured.stableSelection.awaitBestEffort()
    if (!isRestorable(captured.context)) return

    if (captured.context.editor.currentSelection() == null && stableSelection != null) {
      applySelectionBestEffort(captured.context.editor, stableSelection)
    }

    if (!isRestorable(captured.context)) return
    if (captured.context.editor.currentSelection() == null) return
    focusBestEffort(captured.context)
  }

  fun invalidate() {
    clearState(resetContext = true)
  }

  private fun beginEligible(context: EditorContext, selection: Selection) {
    state = State.Eligible(context, selection)
  }

  private fun clearState(resetContext: Boolean) {
    (state as? State.Captured)?.stableSelection?.cancel()
    state = State.Idle
    if (resetContext) {
      currentContext = null
    }
  }

  private fun isRestorable(context: EditorContext): Boolean = currentContext === context

  private fun Editor.currentSelection(): Selection? = state.selection

  private fun focusBestEffort(context: EditorContext) {
    if (!isRestorable(context)) return
    runCatching { focusEditor(context.editor) }
  }

  private suspend fun Deferred<StableSelection?>.awaitBestEffort(): StableSelection? =
    try {
      await()
    } catch (error: CancellationException) {
      currentCoroutineContext().ensureActive()
      null
    } catch (_: Throwable) {
      null
    }

  private suspend fun applySelectionBestEffort(editor: Editor, selection: StableSelection) {
    try {
      applySelection(editor, selection)
    } catch (error: CancellationException) {
      currentCoroutineContext().ensureActive()
    } catch (_: Throwable) {
      // Selection restoration is silent and best-effort.
    }
  }

  private class EditorContext(val editor: Editor)

  private sealed interface State {
    data object Idle : State

    data class Eligible(val context: EditorContext, val selection: Selection) : State

    data class Captured(
      val context: EditorContext,
      val stableSelection: Deferred<StableSelection?>,
    ) : State
  }
}
