package co.typie.editorinput

import android.content.Context
import android.text.InputType
import android.text.Selection
import android.view.KeyCharacterMap
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
  private var cursorX = 0.0
  private var cursorY = 0.0
  private var cursorHeight = 20.0
  private var lastDeleteTime = 0L
  private var batchEditDepth = 0
  private var pendingRestartInput = false
  private var composingRegionLength = 0
  private var lastCommitWasAutocorrect = false

  private val inputMethodManager: InputMethodManager
    get() = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager

  init {
    setOnFocusChangeListener { _, hasFocus ->
      if (!hasFocus) {
        channel.invokeMethod("focusLost", emptyMap<String, Any>())
      }
    }
  }

  private fun commitComposingState() {
    if (isComposing) {
      isComposing = false
      composingText = ""
      channel.invokeMethod("unmarkText", emptyMap<String, Any>())
    }
    composingRegionLength = 0
  }

  private fun cancelComposingState() {
    if (isComposing) {
      isComposing = false
      composingText = ""
      channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
    }
    composingRegionLength = 0
  }

  private fun consumeComposingRegion(): Boolean {
    if (composingRegionLength <= 0) return false
    repeat(composingRegionLength) { performDelete() }
    composingRegionLength = 0
    return true
  }

  private fun scheduleRestartInput() {
    if (batchEditDepth > 0) {
      pendingRestartInput = true
    } else {
      inputMethodManager.restartInput(this)
    }
  }

  private fun performDelete() {
    channel.invokeMethod("deleteBackward", emptyMap<String, Any>())
  }

  private fun performNewline(isShiftPressed: Boolean) {
    commitComposingState()
    if (isShiftPressed) {
      channel.invokeMethod("shortcut", mapOf("action" to "insertHardBreak"))
    } else {
      channel.invokeMethod("performAction", mapOf("action" to "newline"))
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

  fun updateCursor(x: Double, y: Double, height: Double) {
    cursorX = x
    cursorY = y
    cursorHeight = height
  }

  fun resetInputContext() {
    commitComposingState()
    scheduleRestartInput()
  }

  override fun onCheckIsTextEditor(): Boolean = true

  override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
    val meta = event.metaState and (META_CTRL or META_SHIFT or META_ALT)
    val shortcut = SHORTCUTS.find { it.keyCode == keyCode && it.meta == meta }

    if (shortcut != null) {
      commitComposingState()
      channel.invokeMethod("shortcut", mapOf("action" to shortcut.action))
      return true
    }

    if (event.deviceId != KeyCharacterMap.VIRTUAL_KEYBOARD) {
      when (keyCode) {
        KeyEvent.KEYCODE_DEL -> {
          if (isComposing) cancelComposingState() else performDelete()
          return true
        }
        KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> {
          performNewline(event.isShiftPressed)
          return true
        }
      }
    }

    return super.onKeyDown(keyCode, event)
  }

  override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
    outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
    outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_FLAG_NO_ENTER_ACTION or EditorInfo.IME_ACTION_NONE

    return object : BaseInputConnection(this, true) {

      private fun notifyCursorUpdate() {
        val editable = getEditable() ?: return
        inputMethodManager.updateSelection(
          this@EditorInputNativeView,
          Selection.getSelectionStart(editable),
          Selection.getSelectionEnd(editable),
          BaseInputConnection.getComposingSpanStart(editable),
          BaseInputConnection.getComposingSpanEnd(editable)
        )
      }

      private fun handleNewline() {
        commitComposingState()
        super.finishComposingText()
        channel.invokeMethod("performAction", mapOf("action" to "newline"))
        super.commitText("\n", 1)
        notifyCursorUpdate()
      }

      override fun beginBatchEdit(): Boolean {
        batchEditDepth++
        return super.beginBatchEdit()
      }

      override fun endBatchEdit(): Boolean {
        batchEditDepth--
        if (batchEditDepth == 0 && pendingRestartInput) {
          pendingRestartInput = false
          if (!isComposing) {
            inputMethodManager.restartInput(this@EditorInputNativeView)
          }
        }
        return super.endBatchEdit()
      }

      override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val str = text?.toString().orEmpty()

        if (consumeComposingRegion()) {
          if (str.isNotEmpty()) insertTextOrNewline(str)
          super.commitText(text, newCursorPosition)
          notifyCursorUpdate()
          return true
        }

        val skipCacheInvalidation: Boolean
        if (isComposing) {
          skipCacheInvalidation = (str != composingText)
          lastCommitWasAutocorrect = skipCacheInvalidation
          isComposing = false
          composingText = ""
          channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
        } else {
          skipCacheInvalidation = lastCommitWasAutocorrect
          lastCommitWasAutocorrect = false
        }

        if (str.isNotEmpty()) insertTextOrNewline(str)
        super.commitText(text, newCursorPosition)

        if (!skipCacheInvalidation) {
          inputMethodManager.updateSelection(
            this@EditorInputNativeView, 0, 0, -1, -1)
        }
        notifyCursorUpdate()
        return true
      }

      override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val str = text?.toString().orEmpty()

        if (consumeComposingRegion()) {
          if (str.isNotEmpty()) {
            isComposing = true
            composingText = str
            channel.invokeMethod("setMarkedText", mapOf("text" to str))
          }
          super.setComposingText(text, newCursorPosition)
          notifyCursorUpdate()
          return true
        }

        if (str.isEmpty()) {
          cancelComposingState()
        } else {
          isComposing = true
          composingText = str
          channel.invokeMethod("setMarkedText", mapOf("text" to str))
        }

        super.setComposingText(text, newCursorPosition)
        notifyCursorUpdate()
        return true
      }

      override fun finishComposingText(): Boolean {
        composingRegionLength = 0
        if (isComposing) {
          commitComposingState()
          scheduleRestartInput()
        }
        super.finishComposingText()
        notifyCursorUpdate()
        return true
      }

      override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
        if (isComposing) {
          cancelComposingState()
          repeat(beforeLength) { performDelete() }
        } else if (beforeLength > 0) {
          repeat(beforeLength) { performDelete() }
        }
        super.deleteSurroundingText(beforeLength, afterLength)
        notifyCursorUpdate()
        return true
      }

      override fun deleteSurroundingTextInCodePoints(beforeLength: Int, afterLength: Int): Boolean {
        return deleteSurroundingText(beforeLength, afterLength)
      }

      override fun setComposingRegion(start: Int, end: Int): Boolean {
        composingRegionLength = kotlin.math.abs(end - start)
        super.setComposingRegion(start, end)
        return true
      }

      override fun sendKeyEvent(event: KeyEvent): Boolean {
        if (event.action != KeyEvent.ACTION_DOWN) return true

        DPAD_ACTIONS[event.keyCode]?.let { action ->
          commitComposingState()
          super.finishComposingText()
          channel.invokeMethod("shortcut", mapOf("action" to action))
          notifyCursorUpdate()
          return true
        }

        return when (event.keyCode) {
          KeyEvent.KEYCODE_DEL -> {
            if (isComposing) {
              cancelComposingState()
              super.commitText("", 1)
            } else {
              if (event.repeatCount > 0) {
                val now = System.currentTimeMillis()
                if (now - lastDeleteTime < 300) return true
                lastDeleteTime = now
              }
              performDelete()
              super.deleteSurroundingText(1, 0)
            }
            notifyCursorUpdate()
            true
          }
          KeyEvent.KEYCODE_ENTER -> {
            handleNewline()
            true
          }
          KeyEvent.KEYCODE_SPACE -> {
            commitComposingState()
            super.finishComposingText()
            channel.invokeMethod("insertText", mapOf("text" to " "))
            super.commitText(" ", 1)
            notifyCursorUpdate()
            true
          }
          else -> super.sendKeyEvent(event)
        }
      }

      override fun performEditorAction(actionCode: Int): Boolean {
        handleNewline()
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

    private val DPAD_ACTIONS = mapOf(
      KeyEvent.KEYCODE_DPAD_LEFT to "navigateLeft",
      KeyEvent.KEYCODE_DPAD_RIGHT to "navigateRight",
      KeyEvent.KEYCODE_DPAD_UP to "navigateUp",
      KeyEvent.KEYCODE_DPAD_DOWN to "navigateDown",
    )

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
      Shortcut(KeyEvent.KEYCODE_C, META_CTRL, "copy"),
      Shortcut(KeyEvent.KEYCODE_X, META_CTRL, "cut"),
      Shortcut(KeyEvent.KEYCODE_V, META_CTRL, "paste"),
    )
  }
}
