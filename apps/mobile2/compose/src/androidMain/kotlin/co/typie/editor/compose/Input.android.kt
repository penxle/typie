package co.typie.editor.compose

import android.content.Context
import android.text.InputType
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputMethodManager
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.PlatformTextInputMethodRequest
import androidx.compose.ui.platform.PlatformTextInputSessionScope
import co.typie.editor.Editor

@OptIn(ExperimentalComposeUiApi::class)
internal actual suspend fun PlatformTextInputSessionScope.createEditorInputRequest(
  editor: Editor
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
    EditorInputConnection(editor, androidView)
  }
}

@OptIn(ExperimentalComposeUiApi::class)
internal actual fun PlatformTextInputSessionScope.notifyImeSelectionChanged(editor: Editor) {
  val androidView = view
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
