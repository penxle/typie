package co.typie.editorinput

import android.content.Context
import android.text.InputType
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import io.flutter.plugin.common.BinaryMessenger
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

class EditorInputView(
  context: Context,
  messenger: BinaryMessenger,
  viewId: Int
) : io.flutter.plugin.platform.PlatformView, MethodChannel.MethodCallHandler {

  private val channel = MethodChannel(messenger, "co.typie.editor_input.$viewId")
  private val inputView: EditorInputNativeView

  init {
    inputView = EditorInputNativeView(context, channel)
    inputView.isFocusable = true
    inputView.isFocusableInTouchMode = true
    inputView.alpha = 0f
    channel.setMethodCallHandler(this)
  }

  override fun getView(): View = inputView

  override fun dispose() {
    channel.setMethodCallHandler(null)
  }

  override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
    when (call.method) {
      "activate" -> {
        inputView.activate()
        result.success(null)
      }
      "deactivate" -> {
        inputView.deactivate()
        result.success(null)
      }
      "updateCursor" -> {
        result.success(null)
      }
      else -> result.notImplemented()
    }
  }
}

class EditorInputNativeView(
  context: Context,
  private val channel: MethodChannel
) : View(context) {

  private var isComposing = false

  fun activate() {
    requestFocus()
    post {
      val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
      imm.showSoftInput(this, InputMethodManager.SHOW_IMPLICIT)
    }
  }

  fun deactivate() {
    val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
    imm.hideSoftInputFromWindow(windowToken, 0)
    clearFocus()
  }

  override fun onCheckIsTextEditor(): Boolean = true

  override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
    outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
    outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_ACTION_NONE

    return object : BaseInputConnection(this, true) {
      override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
        if (isComposing) {
          isComposing = false
          channel.invokeMethod("unmarkText", emptyMap<String, Any>())
        }
        if (text != null) {
          val str = text.toString()
          if (str == "\n") {
            channel.invokeMethod("performAction", mapOf("action" to "newline"))
          } else {
            channel.invokeMethod("insertText", mapOf("text" to str))
          }
        }
        return true
      }

      override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val str = text?.toString() ?: ""
        if (str.isEmpty()) {
          if (isComposing) {
            isComposing = false
            channel.invokeMethod("unmarkText", emptyMap<String, Any>())
          }
        } else {
          isComposing = true
          channel.invokeMethod("setMarkedText", mapOf("text" to str))
        }
        return true
      }

      override fun finishComposingText(): Boolean {
        if (isComposing) {
          isComposing = false
          channel.invokeMethod("unmarkText", emptyMap<String, Any>())
        }
        return true
      }

      override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
        if (isComposing) {
          isComposing = false
          channel.invokeMethod("unmarkText", emptyMap<String, Any>())
        }
        for (i in 0 until beforeLength) {
          channel.invokeMethod("deleteBackward", emptyMap<String, Any>())
        }
        return true
      }

      override fun performEditorAction(actionCode: Int): Boolean {
        channel.invokeMethod("performAction", mapOf("action" to "newline"))
        return true
      }
    }
  }
}
