package co.typie.editorinput

import android.content.Context
import android.text.InputType
import android.text.Selection
import android.view.KeyCharacterMap
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.CorrectionInfo
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

  private data class PendingRegionSnapshot(
    val length: Int,
    val start: Int,
    val end: Int,
    val seed: String
  )

  private var isComposing = false
  private var composingText = ""
  private var lastDeleteTime = 0L
  private var composingRegionLength = 0
  private var composingRegionStart = -1
  private var composingRegionEnd = -1
  private var composingPrefixToStrip = ""
  private var hasPendingRegionNormalization = false
  private var batchEditDepth = 0
  private var batchEditStartText = ""
  private var batchEditStartSelectionStart = -1
  private var batchEditStartSelectionEnd = -1
  private var batchEditHadBridgedMutation = false
  private var batchEditHadComposingMutation = false

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

  private fun noteBridgedTextMutation() {
    if (batchEditDepth > 0) {
      batchEditHadBridgedMutation = true
    }
  }

  private fun noteComposingMutation() {
    if (batchEditDepth > 0) {
      batchEditHadComposingMutation = true
    }
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

  private fun consumeComposingRegion(requireActiveComposition: Boolean = true): Boolean {
    if (composingRegionLength <= 0) return false

    if (composingPrefixToStrip.isNotEmpty()) {
      clearComposingRegionTracking()
      return false
    }

    if (requireActiveComposition && !isComposing) {
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

  private fun capturePendingRegionSnapshot(): PendingRegionSnapshot? {
    if (!hasPendingRegionNormalization) return null
    if (composingRegionLength <= 0) return null
    return PendingRegionSnapshot(
      length = composingRegionLength,
      start = composingRegionStart,
      end = composingRegionEnd,
      seed = currentComposingRegionText(),
    )
  }

  private fun shouldConsumePendingRegionForReplacement(
    rawText: String,
    pendingRegion: PendingRegionSnapshot
  ): Boolean {
    val seed = pendingRegion.seed
    if (rawText.isEmpty() || seed.isEmpty()) return false
    if (pendingRegion.start < 0 || pendingRegion.end <= pendingRegion.start) return false
    if (seed.length != pendingRegion.length) return false
    if (rawText.startsWith(seed)) return false

    val editable = text ?: return false
    val normalizedStart = pendingRegion.start.coerceIn(0, editable.length)
    val normalizedEnd = pendingRegion.end.coerceIn(0, editable.length)
    if (normalizedEnd <= normalizedStart) return false

    val currentSeed = editable.subSequence(normalizedStart, normalizedEnd).toString()
    if (currentSeed != seed) return false

    val selectionStart = Selection.getSelectionStart(editable)
    val selectionEnd = Selection.getSelectionEnd(editable)
    val selectionInsideRegion = selectionStart in normalizedStart..normalizedEnd && selectionEnd in normalizedStart..normalizedEnd
    if (!selectionInsideRegion) return false

    return true
  }

  private fun consumePendingRegionSnapshot(pendingRegion: PendingRegionSnapshot): Boolean {
    val length = pendingRegion.length
    if (length <= 0) return false
    if (isComposing) return false
    if (composingPrefixToStrip.isNotEmpty()) {
      clearComposingRegionTracking()
      return false
    }

    val editable = text ?: run {
      clearComposingRegionTracking()
      return false
    }
    val normalizedStart = pendingRegion.start.coerceIn(0, editable.length)
    val normalizedEnd = pendingRegion.end.coerceIn(0, editable.length)
    if (normalizedEnd <= normalizedStart) {
      clearComposingRegionTracking()
      return false
    }

    // 커서 이동 후 stale region 삭제 방지
    val currentSeed = editable.subSequence(normalizedStart, normalizedEnd).toString()
    if (currentSeed != pendingRegion.seed) {
      clearComposingRegionTracking()
      return false
    }

    val selectionStart = Selection.getSelectionStart(editable)
    val selectionEnd = Selection.getSelectionEnd(editable)
    val selectionInsideRegion = selectionStart in normalizedStart..normalizedEnd && selectionEnd in normalizedStart..normalizedEnd
    if (!selectionInsideRegion) {
      clearComposingRegionTracking()
      return false
    }

    repeat(length) { performDelete() }
    clearComposingRegionTracking()
    return true
  }

  private fun tryConsumePendingRegionForReplacement(
    rawText: String,
    wasRegionNormalized: Boolean,
    pendingRegion: PendingRegionSnapshot?
  ) {
    if (wasRegionNormalized) return
    if (pendingRegion == null) return
    if (!shouldConsumePendingRegionForReplacement(rawText, pendingRegion)) return
    consumePendingRegionSnapshot(pendingRegion)
  }

  private fun performDelete() {
    noteBridgedTextMutation()
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
    if (text.isNotEmpty()) {
      noteBridgedTextMutation()
    }
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
        noteBridgedTextMutation()
        channel.invokeMethod("performAction", mapOf("action" to "newline"))
        super.commitText("\n", 1)
        notifyCursorUpdate()
      }

      private fun reconcileBatchTextMutation(beforeText: String, afterText: String): Boolean {
        // Gboard 더블 스페이스(. )처럼 batch edit 직접 변경되는 케이스 동기화
        if (beforeText == afterText) return false
        if (isComposing) return false
        if (composingRegionLength > 0 || hasPendingRegionNormalization || composingPrefixToStrip.isNotEmpty()) return false
        if (batchEditStartSelectionStart < 0 || batchEditStartSelectionEnd < 0) return false
        if (batchEditStartSelectionStart != batchEditStartSelectionEnd) return false

        val editable = text ?: return false
        val composingStart = BaseInputConnection.getComposingSpanStart(editable)
        val composingEnd = BaseInputConnection.getComposingSpanEnd(editable)
        if (composingStart >= 0 && composingEnd >= composingStart) return false

        val startCursor = batchEditStartSelectionStart.coerceIn(0, beforeText.length)
        val beforePrefix = beforeText.substring(0, startCursor)
        val beforeSuffix = beforeText.substring(startCursor)
        // 커서 뒤 텍스트가 유지되는 치환만 브리지
        if (!afterText.endsWith(beforeSuffix)) return false
        val coreAfter = afterText.substring(0, afterText.length - beforeSuffix.length)

        var commonPrefix = 0
        val minLength = kotlin.math.min(beforePrefix.length, coreAfter.length)
        while (commonPrefix < minLength && beforePrefix[commonPrefix] == coreAfter[commonPrefix]) {
          commonPrefix += 1
        }

        val removedLength = beforePrefix.length - commonPrefix
        val insertedText = coreAfter.substring(commonPrefix)
        if (removedLength <= 0 && insertedText.isEmpty()) return false

        val selectionStart = Selection.getSelectionStart(editable)
        val selectionEnd = Selection.getSelectionEnd(editable)
        if (selectionStart < 0 || selectionEnd < 0 || selectionStart != selectionEnd) return false

        val expectedCursor = commonPrefix + insertedText.length
        if (selectionStart != expectedCursor) return false

        if (removedLength > 0) {
          noteBridgedTextMutation()
          channel.invokeMethod("replaceBackward", mapOf("length" to removedLength, "text" to insertedText))
          return true
        }

        if (insertedText.isNotEmpty()) {
          insertTextOrNewline(insertedText)
          return true
        }

        return false
      }

      override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val rawStr = text?.toString().orEmpty()
        val pendingRegion = capturePendingRegionSnapshot()
        val str = normalizeSeededComposingText(rawStr)
        val wasRegionNormalized = (str != rawStr)
        val superText: CharSequence? = if (str == rawStr) text else str
        val isSingleWhitespaceCommit = str.length == 1 && str[0].isWhitespace()
        var shouldInsertText = str.isNotEmpty()

        tryConsumePendingRegionForReplacement(rawStr, wasRegionNormalized, pendingRegion)

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
        noteComposingMutation()
        // 디자인키보드의 천지인 키보드는 조합 문자열 전체(예: '살ㅇ' -> '살이' -> '살아')를 넘기므로
        // setComposingText에서는 seed prefix 제거/삭제 브리지 없이 원문 조합을 그대로 반영한다.
        val str = rawStr
        val superText: CharSequence? = text

        // 문제 시나리오:
        // - LG 키보드에서 "안녕 " 입력 후 백스페이스를 누르면
        //   setComposingRegion(0..2) + setComposingText("안녕")으로 재조합을 시작한다.
        // - 이때 기존 범위를 먼저 소비하지 않으면 preedit이 뒤에 붙어 "안녕안녕"이 된다.
        val shouldConsumeRegionOnComposeStart = !isComposing && str.isNotEmpty() && composingRegionLength > 0
        if (shouldConsumeRegionOnComposeStart) {
          consumeComposingRegion(requireActiveComposition = false)
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

      override fun beginBatchEdit(): Boolean {
        if (batchEditDepth == 0) {
          batchEditStartText = text?.toString().orEmpty()
          val editable = text
          batchEditStartSelectionStart = editable?.let { Selection.getSelectionStart(it) } ?: -1
          batchEditStartSelectionEnd = editable?.let { Selection.getSelectionEnd(it) } ?: -1
          batchEditHadBridgedMutation = false
          batchEditHadComposingMutation = false
        }
        batchEditDepth += 1
        return super.beginBatchEdit()
      }

      override fun endBatchEdit(): Boolean {
        val result = super.endBatchEdit()
        if (batchEditDepth > 0) {
          batchEditDepth -= 1
        }
        if (batchEditDepth == 0) {
          val finalText = text?.toString().orEmpty()
          // 배치 내 브리지 반영이 있으면 중복 반영 금지
          // 조합 문자열 업데이트(setComposingText)로 바뀐 shadow text는 에디터 본문 변경으로 재해석하지 않는다.
          val shouldReconcile = !batchEditHadBridgedMutation && !batchEditHadComposingMutation && finalText != batchEditStartText
          if (shouldReconcile) {
            reconcileBatchTextMutation(batchEditStartText, finalText)
          }
          batchEditStartText = finalText
          batchEditStartSelectionStart = -1
          batchEditStartSelectionEnd = -1
          batchEditHadBridgedMutation = false
          batchEditHadComposingMutation = false
        }
        return result
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

      override fun commitCorrection(correctionInfo: CorrectionInfo?): Boolean {
        val oldText = correctionInfo?.oldText?.toString().orEmpty()
        val newText = correctionInfo?.newText?.toString().orEmpty()

        // - "안넝" 입력 후 스페이스 자동수정이 걸리면 같은 batch에서
        //   commitText("안녕") 직후 commitCorrection(old="", new="안녕")가 연달아 올 수 있다.
        // - commitText에서 이미 브리지 삽입을 보냈다면 commitCorrection 삽입은 중복이다.
        val shouldSkipInsertedCorrection =
          oldText.isEmpty() &&
          newText.isNotEmpty() &&
          batchEditDepth > 0 &&
          batchEditHadBridgedMutation
        if (shouldSkipInsertedCorrection) {
          val result = super.commitCorrection(correctionInfo)
          notifyCursorUpdate()
          return result
        }

        if (oldText.isNotEmpty()) {
          noteBridgedTextMutation()
          channel.invokeMethod("replaceBackward", mapOf("length" to oldText.length, "text" to newText))
        } else if (newText.isNotEmpty()) {
          insertTextOrNewline(newText)
        }

        val result = super.commitCorrection(correctionInfo)
        notifyCursorUpdate()
        return result
      }

      override fun setComposingRegion(start: Int, end: Int): Boolean {
        noteComposingMutation()
        composingRegionStart = kotlin.math.min(start, end).coerceAtLeast(0)
        composingRegionEnd = kotlin.math.max(start, end).coerceAtLeast(0)
        composingRegionLength = (composingRegionEnd - composingRegionStart).coerceAtLeast(0)
        hasPendingRegionNormalization = composingRegionLength > 0
        super.setComposingRegion(start, end)
        return true
      }

      override fun getTextBeforeCursor(maxChars: Int, flags: Int): CharSequence {
        val before = super.getTextBeforeCursor(maxChars, flags)
        return if (before.isNullOrEmpty()) " " else before
      }

      override fun getTextAfterCursor(maxChars: Int, flags: Int): CharSequence {
        val after = super.getTextAfterCursor(maxChars, flags)
        return if (after.isNullOrEmpty()) " " else after
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

        resolveNavigationDirection(event)?.let { direction ->
          commitComposingState()
          super.finishComposingText()
          channel.invokeMethod(
            "navigate",
            mapOf(
              "direction" to direction,
              "extend" to event.isShiftPressed
            )
          )
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
            insertTextOrNewline(" ")
            super.commitText(" ", 1)
            notifyCursorUpdate()
            true
          }
          else -> {
            val unicode = event.unicodeChar
            val isHardwarePrintableKey =
              event.deviceId != KeyCharacterMap.VIRTUAL_KEYBOARD &&
              unicode >= 0x20 &&
              (unicode and KeyCharacterMap.COMBINING_ACCENT == 0) &&
              !event.isCtrlPressed &&
              !event.isAltPressed &&
              !event.isMetaPressed
            if (isHardwarePrintableKey) {
              val text = String(Character.toChars(unicode))
              commitComposingState()
              super.finishComposingText()
              insertTextOrNewline(text)
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

  private fun resolveNavigationDirection(event: KeyEvent): String? {
    return when (event.keyCode) {
      KeyEvent.KEYCODE_DPAD_LEFT ->
        if (event.isMetaPressed) "lineStart" else if (event.isCtrlPressed) "wordLeft" else "left"
      KeyEvent.KEYCODE_DPAD_RIGHT ->
        if (event.isMetaPressed) "lineEnd" else if (event.isCtrlPressed) "wordRight" else "right"
      KeyEvent.KEYCODE_DPAD_UP ->
        if (event.isMetaPressed) "documentStart" else if (event.isAltPressed) "sentenceUp" else "up"
      KeyEvent.KEYCODE_DPAD_DOWN ->
        if (event.isMetaPressed) "documentEnd" else if (event.isAltPressed) "sentenceDown" else "down"
      KeyEvent.KEYCODE_MOVE_HOME ->
        if (event.isCtrlPressed) "documentStart" else "lineStart"
      KeyEvent.KEYCODE_MOVE_END ->
        if (event.isCtrlPressed) "documentEnd" else "lineEnd"
      KeyEvent.KEYCODE_PAGE_UP -> "pageUp"
      KeyEvent.KEYCODE_PAGE_DOWN -> "pageDown"
      else -> null
    }
  }
}
