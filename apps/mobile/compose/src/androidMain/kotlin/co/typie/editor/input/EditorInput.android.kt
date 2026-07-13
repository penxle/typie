package co.typie.editor.input

import android.content.Context
import android.os.Looper
import android.text.InputType
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import androidx.compose.ui.text.input.EditCommand
import co.typie.editor.Editor
import co.typie.editor.scroll.EditorBringIntoViewRequests

@OptIn(ExperimentalComposeUiApi::class)
internal actual suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor,
  bringIntoViewRequests: EditorBringIntoViewRequests,
  onEditCommand: (List<EditCommand>) -> Unit,
  focusedRectInRoot: () -> Rect?,
  textFieldRectInRoot: () -> Rect?,
  textClippingRectInRoot: () -> Rect?,
  suppressSoftwareKeyboard: Boolean,
): PlatformTextInputMethodRequest {
  val androidView = view
  return PlatformTextInputMethodRequest { outAttrs ->
    outAttrs.inputType =
      InputType.TYPE_CLASS_TEXT or
        InputType.TYPE_TEXT_FLAG_MULTI_LINE or
        InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
    outAttrs.imeOptions = EditorInfo.IME_ACTION_NONE or EditorInfo.IME_FLAG_NO_EXTRACT_UI
    val ctx = editor.tickIme
    outAttrs.initialSelStart = ctx?.selection?.start ?: -1
    outAttrs.initialSelEnd = ctx?.selection?.end ?: -1
    val connection =
      EditorInputConnection(
        editor = editor,
        view = androidView,
        bringIntoViewRequests = bringIntoViewRequests,
      )
    if (suppressSoftwareKeyboard) {
      androidView.post { hideEditorSoftwareKeyboard(androidView) }
    }
    editor.inputRecorder?.record { seq, t ->
      RecordedInputEntry.SessionStart(
        seq = seq,
        t = t,
        initialSelStart = outAttrs.initialSelStart,
        initialSelEnd = outAttrs.initialSelEnd,
      )
    }
    connection
  }
}

internal actual fun requiresEditorInputSessionRestartForSoftwareKeyboardSuppression(): Boolean =
  false

@OptIn(ExperimentalComposeUiApi::class)
internal actual fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor) {
  val androidView = view

  fun update() {
    val imm =
      androidView.context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager
        ?: return
    val ctx = editor.tickIme
    val selStart = ctx?.selection?.start ?: -1
    val selEnd = ctx?.selection?.end ?: -1
    val composingStart = ctx?.composing?.start ?: -1
    val composingEnd = ctx?.composing?.end ?: -1
    editor.inputRecorder?.record { seq, t ->
      RecordedInputEntry.UpdateSelection(
        seq = seq,
        t = t,
        selStart = selStart,
        selEnd = selEnd,
        composingStart = composingStart,
        composingEnd = composingEnd,
      )
    }
    imm.updateSelection(androidView, selStart, selEnd, composingStart, composingEnd)
  }

  if (Looper.myLooper() == Looper.getMainLooper()) {
    update()
  } else {
    androidView.post { update() }
  }
}

private fun hideEditorSoftwareKeyboard(view: View) {
  val imm =
    view.context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager ?: return
  imm.hideSoftInputFromWindow(view.windowToken, 0)
}
