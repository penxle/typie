package co.typie.editorinput

import android.content.Context
import android.text.InputType
import android.view.KeyEvent
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
  private val inputView = EditorInputNativeView(context, channel).apply {
    isFocusable = true
    isFocusableInTouchMode = true
    alpha = 0f
  }

  init {
    channel.setMethodCallHandler(this)
  }

  override fun getView(): View = inputView

  override fun dispose() {
    channel.setMethodCallHandler(null)
  }

  override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
    when (call.method) {
      "activate" -> inputView.activate()
      "deactivate" -> inputView.deactivate()
      "releaseFocus" -> inputView.releaseFocus()
      "resetInputContext" -> inputView.resetInputContext()
      "updateCursor" -> {
        (call.arguments as? Map<*, *>)?.let { args ->
          inputView.updateCursor(
            x = (args["x"] as? Number)?.toDouble() ?: 0.0,
            y = (args["y"] as? Number)?.toDouble() ?: 0.0,
            height = (args["height"] as? Number)?.toDouble() ?: 20.0
          )
        }
      }
      else -> {
        result.notImplemented()
        return
      }
    }
    result.success(null)
  }
}

class EditorInputNativeView(
  context: Context,
  private val channel: MethodChannel
) : View(context) {

  private var isComposing = false
  private var composingText = ""
  private var finishedComposingText: String? = null
  private var cursorX = 0.0
  private var cursorY = 0.0
  private var cursorHeight = 20.0

  private val inputMethodManager: InputMethodManager
    get() = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager

  init {
    setOnFocusChangeListener { _, hasFocus ->
      if (!hasFocus) {
        channel.invokeMethod("focusLost", emptyMap<String, Any>())
      }
    }
  }

  private val hasComposingState: Boolean
    get() = isComposing || finishedComposingText != null

  private fun commitComposingState() {
    if (hasComposingState) {
      isComposing = false
      composingText = ""
      finishedComposingText = null
      channel.invokeMethod("unmarkText", emptyMap<String, Any>())
    }
  }

  private fun cancelComposingState() {
    if (hasComposingState) {
      isComposing = false
      composingText = ""
      finishedComposingText = null
      channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
    }
  }

  private fun insertTextOrNewline(text: String) {
    if (text == "\n") {
      channel.invokeMethod("performAction", mapOf("action" to "newline"))
    } else {
      channel.invokeMethod("insertText", mapOf("text" to text))
    }
  }

  fun activate() {
    requestFocus()
    post { inputMethodManager.showSoftInput(this, InputMethodManager.SHOW_IMPLICIT) }
  }

  fun deactivate() {
    inputMethodManager.hideSoftInputFromWindow(windowToken, 0)
    clearFocus()
  }

  fun releaseFocus() {
    clearFocus()
  }

  fun updateCursor(x: Double, y: Double, height: Double) {
    cursorX = x
    cursorY = y
    cursorHeight = height
  }

  fun resetInputContext() {
    commitComposingState()
    inputMethodManager.restartInput(this)
  }

  override fun onCheckIsTextEditor(): Boolean = true

  override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
    val meta = event.metaState and (META_CTRL or META_SHIFT or META_ALT)
    val shortcut = SHORTCUTS.find { it.keyCode == keyCode && it.meta == meta }

    return if (shortcut != null) {
      commitComposingState()
      channel.invokeMethod("shortcut", mapOf("action" to shortcut.action))
      true
    } else {
      super.onKeyDown(keyCode, event)
    }
  }

  override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
    outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
    outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_ACTION_NONE

    return object : BaseInputConnection(this, false) {
      override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val str = text?.toString().orEmpty()
        val prevComposing = finishedComposingText ?: composingText
        val wasComposing = hasComposingState

        if (wasComposing) {
          isComposing = false
          finishedComposingText = null
          channel.invokeMethod("unmarkText", emptyMap<String, Any>())

          val remaining = if (str.startsWith(prevComposing)) {
            str.substring(prevComposing.length)
          } else {
            str
          }
          composingText = ""

          if (remaining.isNotEmpty()) {
            insertTextOrNewline(remaining)
          }
          return true
        }

        composingText = ""
        if (str.isNotEmpty()) {
          insertTextOrNewline(str)
        }
        return true
      }

      override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val str = text?.toString().orEmpty()

        if (finishedComposingText != null) {
          channel.invokeMethod("unmarkText", emptyMap<String, Any>())
          finishedComposingText = null
        }

        if (str.isEmpty()) {
          if (isComposing) {
            isComposing = false
            composingText = ""
            channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
          }
        } else {
          isComposing = true
          composingText = str
          channel.invokeMethod("setMarkedText", mapOf("text" to str))
        }
        return true
      }

      override fun finishComposingText(): Boolean {
        if (isComposing) {
          isComposing = false
          finishedComposingText = composingText
          composingText = ""
        }
        return true
      }

      override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
        if (finishedComposingText != null) {
          channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
          finishedComposingText = null
          return true
        }
        if (isComposing) {
          isComposing = false
          composingText = ""
          channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
          return true
        }
        repeat(beforeLength) {
          channel.invokeMethod("deleteBackward", emptyMap<String, Any>())
        }
        return true
      }

      override fun getTextBeforeCursor(maxChars: Int, flags: Int): CharSequence = " "

      override fun getTextAfterCursor(maxChars: Int, flags: Int): CharSequence = ""

      override fun getSelectedText(flags: Int): CharSequence? = null

      override fun sendKeyEvent(event: KeyEvent): Boolean {
        if (event.action != KeyEvent.ACTION_DOWN) {
          return super.sendKeyEvent(event)
        }

        return when (event.keyCode) {
          KeyEvent.KEYCODE_DEL -> {
            when {
              finishedComposingText != null -> {
                channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
                finishedComposingText = null
              }
              isComposing -> {
                isComposing = false
                composingText = ""
                channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
              }
              else -> {
                channel.invokeMethod("deleteBackward", emptyMap<String, Any>())
              }
            }
            true
          }
          KeyEvent.KEYCODE_ENTER -> {
            commitComposingState()
            channel.invokeMethod("performAction", mapOf("action" to "newline"))
            true
          }
          else -> super.sendKeyEvent(event)
        }
      }

      override fun performEditorAction(actionCode: Int): Boolean {
        commitComposingState()
        channel.invokeMethod("performAction", mapOf("action" to "newline"))
        return true
      }
    }
  }

  private data class Shortcut(val keyCode: Int, val meta: Int, val action: String)

  companion object {
    private const val META_CTRL = KeyEvent.META_CTRL_ON
    private const val META_SHIFT = KeyEvent.META_SHIFT_ON
    private const val META_ALT = KeyEvent.META_ALT_ON
    private const val META_CTRL_SHIFT = META_CTRL or META_SHIFT

    private val SHORTCUTS = listOf(
      Shortcut(KeyEvent.KEYCODE_A, META_CTRL, "selectAll"),
      Shortcut(KeyEvent.KEYCODE_B, META_CTRL, "toggleBold"),
      Shortcut(KeyEvent.KEYCODE_I, META_CTRL, "toggleItalic"),
      Shortcut(KeyEvent.KEYCODE_U, META_CTRL, "toggleUnderline"),
      Shortcut(KeyEvent.KEYCODE_S, META_CTRL_SHIFT, "toggleStrikethrough"),
      Shortcut(KeyEvent.KEYCODE_Z, META_CTRL, "undo"),
      Shortcut(KeyEvent.KEYCODE_Z, META_CTRL_SHIFT, "redo"),
      Shortcut(KeyEvent.KEYCODE_BACKSLASH, META_CTRL, "clearFormatting"),
      Shortcut(KeyEvent.KEYCODE_TAB, 0, "indent"),
      Shortcut(KeyEvent.KEYCODE_TAB, META_SHIFT, "outdent"),
      Shortcut(KeyEvent.KEYCODE_ENTER, META_CTRL, "insertPageBreak"),
      Shortcut(KeyEvent.KEYCODE_ENTER, META_SHIFT, "insertHardBreak"),
      Shortcut(KeyEvent.KEYCODE_DEL, META_CTRL, "deleteToLineStart"),
      Shortcut(KeyEvent.KEYCODE_DEL, META_ALT, "deleteWordBackward"),
    )
  }
}
