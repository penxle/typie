package co.typie.editor.input

import android.content.Context
import android.os.Looper
import android.text.InputType
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import androidx.compose.ui.ExperimentalComposeUiApi
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
  suppressSoftwareKeyboard: Boolean,
): PlatformTextInputMethodRequest {
  val androidView = view
  return PlatformTextInputMethodRequest { outAttrs ->
    outAttrs.inputType =
      InputType.TYPE_CLASS_TEXT or
        InputType.TYPE_TEXT_FLAG_MULTI_LINE or
        InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
    outAttrs.imeOptions = EditorInfo.IME_ACTION_NONE or EditorInfo.IME_FLAG_NO_EXTRACT_UI
    val ctx = editor.ime(0, 0)
    outAttrs.initialSelStart = ctx.selection.start
    outAttrs.initialSelEnd = ctx.selection.end
    val connection =
      EditorInputConnection(
        editor = editor,
        view = androidView,
        bringIntoViewRequests = bringIntoViewRequests,
      )
    if (suppressSoftwareKeyboard) {
      androidView.post { hideEditorSoftwareKeyboard(androidView) }
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
    val ctx = editor.ime(0, 0)
    val composingStart = ctx.composing?.start ?: -1
    val composingEnd = ctx.composing?.end ?: -1
    imm.updateSelection(
      androidView,
      ctx.selection.start,
      ctx.selection.end,
      composingStart,
      composingEnd,
    )
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
