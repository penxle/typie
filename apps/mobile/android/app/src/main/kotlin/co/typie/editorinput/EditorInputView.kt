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
import android.view.inputmethod.InputConnectionWrapper
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
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
) : EditText(context) {

  private var isComposing = false
  private var composingText = ""
  private var lastDeleteTime = 0L
  private var composingRegionLength = 0
  private var composingRegionStart = -1
  private var composingRegionEnd = -1
  private var composingPrefixToStrip = ""
  private var hasPendingRegionNormalization = false

  private val inputMethodManager: InputMethodManager
    get() = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager

  private fun clearComposingRegionTracking() {
    composingRegionLength = 0
    composingRegionStart = -1
    composingRegionEnd = -1
    hasPendingRegionNormalization = false
  }

  private fun clearComposingPrefixTracking() {
    composingPrefixToStrip = ""
  }

  private fun currentComposingRegionText(): String {
    val editable = text ?: return ""
    if (composingRegionStart < 0 || composingRegionEnd < 0) return ""
    val start = composingRegionStart.coerceIn(0, editable.length)
    val end = composingRegionEnd.coerceIn(0, editable.length)
    if (end <= start) return ""
    return editable.subSequence(start, end).toString()
  }

  private fun normalizeSeededComposingText(text: String): String {
    if (text.isEmpty()) return text

    val activePrefix = composingPrefixToStrip
    if (activePrefix.isNotEmpty()) {
      if (text.startsWith(activePrefix)) {
        return text.substring(activePrefix.length.coerceAtMost(text.length))
      }
      clearComposingPrefixTracking()
    }

    if (!hasPendingRegionNormalization || composingRegionLength <= 0) return text
    hasPendingRegionNormalization = false

    val seed = currentComposingRegionText()
    if (seed.isEmpty()) {
      clearComposingRegionTracking()
      return text
    }

    if (text.length > seed.length && text.startsWith(seed)) {
      val normalized = text.substring(seed.length)
      if (normalized.isNotEmpty()) {
        composingPrefixToStrip = seed
      }
      return normalized
    }

    clearComposingRegionTracking()
    clearComposingPrefixTracking()
    return text
  }

  init {
    setText("")
    setSelection(0)
    isCursorVisible = false
    inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
    imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_FLAG_NO_ENTER_ACTION or EditorInfo.IME_ACTION_NONE
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
    clearComposingRegionTracking()
    clearComposingPrefixTracking()
  }

  private fun cancelComposingState() {
    if (isComposing) {
      isComposing = false
      composingText = ""
      channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
    }
    clearComposingRegionTracking()
    clearComposingPrefixTracking()
  }

  private fun consumeComposingRegion(): Boolean {
    if (composingRegionLength <= 0) return false

    if (composingPrefixToStrip.isNotEmpty()) {
      clearComposingRegionTracking()
      return false
    }

    if (!isComposing) {
      return false
    }

    val editable = text
    if (editable == null) {
      clearComposingRegionTracking()
      return false
    }

    val composingStart = BaseInputConnection.getComposingSpanStart(editable)
    val composingEnd = BaseInputConnection.getComposingSpanEnd(editable)
    val hasComposingSpan = composingStart >= 0 && composingEnd >= composingStart

    val selectionStart = Selection.getSelectionStart(editable)
    val selectionEnd = Selection.getSelectionEnd(editable)
    val selectionInsideComposing =
      hasComposingSpan &&
      selectionStart in composingStart..composingEnd &&
      selectionEnd in composingStart..composingEnd

    if (hasComposingSpan && !selectionInsideComposing) {
      clearComposingRegionTracking()
      return false
    }

    if (!hasComposingSpan) {
      clearComposingRegionTracking()
      return false
    }

    repeat(composingRegionLength) { performDelete() }
    clearComposingRegionTracking()
    return true
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
  }

  fun resetInputContext() {
    commitComposingState()
    inputMethodManager.restartInput(this)
  }

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
    val target = super.onCreateInputConnection(outAttrs) ?: return BaseInputConnection(this, true)

    return object : InputConnectionWrapper(target, true) {

      private fun notifyCursorUpdate() {
        val editable = text ?: return
        inputMethodManager.updateSelection(
          this@EditorInputNativeView,
          Selection.getSelectionStart(editable),
          Selection.getSelectionEnd(editable),
          BaseInputConnection.getComposingSpanStart(editable),
          BaseInputConnection.getComposingSpanEnd(editable)
        )
      }

      private fun finishComposingNow(): Boolean {
        clearComposingRegionTracking()
        clearComposingPrefixTracking()
        if (isComposing) {
          commitComposingState()
        }
        val result = super.finishComposingText()
        notifyCursorUpdate()
        return result
      }

      private fun handleNewline() {
        commitComposingState()
        super.finishComposingText()
        channel.invokeMethod("performAction", mapOf("action" to "newline"))
        super.commitText("\n", 1)
        notifyCursorUpdate()
      }

      override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val rawStr = text?.toString().orEmpty()
        val str = normalizeSeededComposingText(rawStr)
        val wasRegionNormalized = (str != rawStr)
        val superText: CharSequence? = if (str == rawStr) text else str
        val isSingleWhitespaceCommit = str.length == 1 && str[0].isWhitespace()
        var shouldInsertText = str.isNotEmpty()

        if (!wasRegionNormalized && consumeComposingRegion()) {
          if (str.isNotEmpty()) insertTextOrNewline(str)
          super.commitText(superText, newCursorPosition)
          notifyCursorUpdate()
          return true
        }
        if (wasRegionNormalized) {
          clearComposingRegionTracking()
        }

        if (isComposing && isSingleWhitespaceCommit) {
          commitComposingState()
        } else if (isComposing && str == composingText) {
          commitComposingState()
          shouldInsertText = false
        } else if (isComposing) {
          isComposing = false
          composingText = ""
          clearComposingRegionTracking()
          clearComposingPrefixTracking()
          channel.invokeMethod("cancelMarkedText", emptyMap<String, Any>())
        }

        if (shouldInsertText) insertTextOrNewline(str)
        super.commitText(superText, newCursorPosition)

        notifyCursorUpdate()
        return true
      }

      override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val rawStr = text?.toString().orEmpty()
        val str = normalizeSeededComposingText(rawStr)
        val wasRegionNormalized = (str != rawStr)
        val superText: CharSequence? = if (str == rawStr) text else str

        if (!wasRegionNormalized && consumeComposingRegion()) {
          if (str.isNotEmpty()) {
            isComposing = true
            composingText = str
            channel.invokeMethod("setMarkedText", mapOf("text" to str))
          }
          super.setComposingText(superText, newCursorPosition)
          notifyCursorUpdate()
          return true
        }
        if (wasRegionNormalized) {
          clearComposingRegionTracking()
        }

        if (str.isEmpty()) {
          cancelComposingState()
        } else {
          isComposing = true
          composingText = str
          channel.invokeMethod("setMarkedText", mapOf("text" to str))
        }

        super.setComposingText(superText, newCursorPosition)
        notifyCursorUpdate()
        return true
      }

      override fun finishComposingText(): Boolean {
        return finishComposingNow()
      }

      override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
        if (isComposing) {
          cancelComposingState()
          super.commitText("", 1)
          notifyCursorUpdate()
          return true
        }
        if (beforeLength > 0) {
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
        composingRegionStart = kotlin.math.min(start, end).coerceAtLeast(0)
        composingRegionEnd = kotlin.math.max(start, end).coerceAtLeast(0)
        composingRegionLength = (composingRegionEnd - composingRegionStart).coerceAtLeast(0)
        hasPendingRegionNormalization = composingRegionLength > 0
        super.setComposingRegion(start, end)
        return true
      }

      override fun setSelection(start: Int, end: Int): Boolean {
        val editable = text
        val composingStart = editable?.let { BaseInputConnection.getComposingSpanStart(it) } ?: -1
        val composingEnd = editable?.let { BaseInputConnection.getComposingSpanEnd(it) } ?: -1
        val hasComposingSpan = composingStart >= 0 && composingEnd >= composingStart
        val movedOutsideComposing =
          hasComposingSpan && (start < composingStart || start > composingEnd || end < composingStart || end > composingEnd)

        if (composingRegionLength > 0 && (!hasComposingSpan || movedOutsideComposing)) {
          clearComposingRegionTracking()
        }

        if (isComposing && movedOutsideComposing) {
          cancelComposingState()
        }

        val result = super.setSelection(start, end)
        notifyCursorUpdate()
        return result
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
            if (event.deviceId == KeyCharacterMap.VIRTUAL_KEYBOARD) {
              return super.sendKeyEvent(event)
            }
            commitComposingState()
            super.finishComposingText()
            channel.invokeMethod("insertText", mapOf("text" to " "))
            super.commitText(" ", 1)
            notifyCursorUpdate()
            true
          }
          else -> {
            val isNumberKey =
              (event.keyCode in KeyEvent.KEYCODE_0..KeyEvent.KEYCODE_9) ||
              (event.keyCode in KeyEvent.KEYCODE_NUMPAD_0..KeyEvent.KEYCODE_NUMPAD_9)
            val unicode = event.unicodeChar
            if (
              isNumberKey &&
              unicode != 0 &&
              !event.isCtrlPressed &&
              !event.isAltPressed &&
              !event.isMetaPressed
            ) {
              val text = unicode.toChar().toString()
              commitComposingState()
              super.finishComposingText()
              channel.invokeMethod("insertText", mapOf("text" to text))
              super.commitText(text, 1)
              notifyCursorUpdate()
              return true
            }

            super.sendKeyEvent(event)
          }
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
