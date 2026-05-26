package co.typie.editor.input

import android.os.Bundle
import android.os.CancellationSignal
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.CompletionInfo
import android.view.inputmethod.CorrectionInfo
import android.view.inputmethod.ExtractedText
import android.view.inputmethod.ExtractedTextRequest
import android.view.inputmethod.HandwritingGesture
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputContentInfo
import android.view.inputmethod.PreviewableHandwritingGesture
import android.view.inputmethod.SurroundingText
import android.view.inputmethod.TextAttribute
import android.view.inputmethod.TextSnapshot
import androidx.annotation.RequiresApi
import co.typie.editor.Editor
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent as FfiKeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.scroll.EditorBringIntoViewRequests
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.syncWithBringIntoView
import java.util.concurrent.Executor
import java.util.function.IntConsumer

internal class EditorInputConnection(
  private val editor: Editor,
  private val view: View,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
) : InputConnection {
  private val batch = ImeEditBatch { messages ->
    editor.syncWithBringIntoView(bringIntoViewRequests) {
      for (message in messages) {
        enqueue(message)
      }
      beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentCursorLine) }
    }
  }

  override fun getTextBeforeCursor(n: Int, flags: Int): CharSequence? {
    if (n < 0) return null
    val ctx = editor.ime(n, 0)
    return ctx.text.substring(0, ctx.selection.start - ctx.windowStart)
  }

  override fun getTextAfterCursor(n: Int, flags: Int): CharSequence? {
    if (n < 0) return null
    val ctx = editor.ime(0, n)
    return ctx.text.substring(ctx.selection.end - ctx.windowStart)
  }

  override fun getSelectedText(flags: Int): CharSequence? {
    val ctx = editor.ime(0, 0)
    val start = ctx.selection.start - ctx.windowStart
    val end = ctx.selection.end - ctx.windowStart
    val text = ctx.text.substring(start, end)
    return text.ifEmpty { null }
  }

  @RequiresApi(31)
  override fun getSurroundingText(
    beforeLength: Int,
    afterLength: Int,
    flags: Int,
  ): SurroundingText? {
    if (beforeLength < 0 || afterLength < 0) return null
    val ctx = editor.ime(beforeLength, afterLength)
    val selStart = ctx.selection.start - ctx.windowStart
    val selEnd = ctx.selection.end - ctx.windowStart
    return SurroundingText(ctx.text, selStart, selEnd, ctx.windowStart)
  }

  override fun getCursorCapsMode(reqModes: Int): Int = 0

  override fun getExtractedText(request: ExtractedTextRequest?, flags: Int): ExtractedText? = null

  override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
    val value = text?.toString() ?: return false
    if (value == "\n") {
      batch.enqueue(Message.Key(FfiKeyEvent(Key.Enter)))
    } else {
      batch.enqueue(FlatImeOp.ReplaceSelection(value))
    }
    return true
  }

  override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
    val value = text?.toString() ?: return false
    batch.enqueue(FlatImeOp.Compose(value))
    return true
  }

  override fun setComposingRegion(start: Int, end: Int): Boolean {
    batch.enqueue(FlatImeOp.SetComposition(start, end))
    return true
  }

  override fun finishComposingText(): Boolean {
    batch.enqueue(FlatImeOp.ClearComposition)
    return true
  }

  override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
    batch.enqueue(FlatImeOp.DeleteSurroundingUtf16(beforeLength, afterLength))
    return true
  }

  override fun deleteSurroundingTextInCodePoints(beforeLength: Int, afterLength: Int): Boolean {
    batch.enqueue(FlatImeOp.DeleteSurrounding(beforeLength, afterLength))
    return true
  }

  override fun setSelection(start: Int, end: Int): Boolean {
    batch.enqueue(FlatImeOp.SetSelection(start, end))
    return true
  }

  override fun sendKeyEvent(event: KeyEvent?): Boolean {
    if (event == null || event.action != KeyEvent.ACTION_DOWN) return false
    val key =
      when (event.keyCode) {
        KeyEvent.KEYCODE_DEL -> Key.Backspace
        KeyEvent.KEYCODE_FORWARD_DEL -> Key.Delete
        KeyEvent.KEYCODE_ENTER -> Key.Enter
        KeyEvent.KEYCODE_TAB -> Key.Tab
        KeyEvent.KEYCODE_ESCAPE -> Key.Escape
        else -> return false
      }
    batch.enqueue(Message.Key(FfiKeyEvent(key)))
    return true
  }

  override fun performEditorAction(editorAction: Int): Boolean = false

  override fun performContextMenuAction(id: Int): Boolean = false

  override fun beginBatchEdit(): Boolean = batch.beginBatchEdit()

  override fun endBatchEdit(): Boolean = batch.endBatchEdit()

  override fun clearMetaKeyStates(states: Int): Boolean = false

  override fun reportFullscreenMode(enabled: Boolean): Boolean = false

  override fun performPrivateCommand(action: String?, data: Bundle?): Boolean = false

  override fun requestCursorUpdates(cursorUpdateMode: Int): Boolean = false

  override fun getHandler() = null

  override fun closeConnection() {
    batch.closeConnection()
  }

  override fun commitContent(
    inputContentInfo: InputContentInfo,
    flags: Int,
    opts: Bundle?,
  ): Boolean = false

  override fun commitCompletion(text: CompletionInfo?): Boolean = false

  override fun commitCorrection(correctionInfo: CorrectionInfo?): Boolean = false

  override fun performSpellCheck(): Boolean = false

  override fun performHandwritingGesture(
    gesture: HandwritingGesture,
    executor: Executor?,
    consumer: IntConsumer?,
  ) {}

  override fun previewHandwritingGesture(
    gesture: PreviewableHandwritingGesture,
    cancellationSignal: CancellationSignal?,
  ): Boolean = false

  override fun replaceText(
    start: Int,
    end: Int,
    text: CharSequence,
    newCursorPosition: Int,
    textAttribute: TextAttribute?,
  ): Boolean = false

  override fun takeSnapshot(): TextSnapshot? = null

  override fun setImeConsumesInput(imeConsumesInput: Boolean): Boolean = false
}

private class ImeEditBatch(private val dispatch: (List<Message>) -> Unit) {
  private var batchLevel = 0
  private val pendingOps = mutableListOf<FlatImeOp>()
  private val pendingMessages = mutableListOf<Message>()

  fun beginBatchEdit(): Boolean {
    batchLevel++
    return true
  }

  fun endBatchEdit(): Boolean {
    if (batchLevel > 0) batchLevel--
    flushIfReady()
    return batchLevel > 0
  }

  fun closeConnection() {
    batchLevel = 0
    enqueue(FlatImeOp.ClearComposition)
  }

  fun enqueue(op: FlatImeOp) {
    pendingOps.add(op)
    flushIfReady()
  }

  fun enqueue(message: Message) {
    flushOpsToPendingMessages()
    pendingMessages.add(message)
    flushIfReady()
  }

  private fun flushIfReady() {
    if (batchLevel == 0) flush()
  }

  private fun flush() {
    flushOpsToPendingMessages()
    if (pendingMessages.isEmpty()) return

    val messages = pendingMessages.toList()
    pendingMessages.clear()
    dispatch(messages)
  }

  private fun flushOpsToPendingMessages() {
    if (pendingOps.isEmpty()) return
    pendingMessages.add(Message.TextInput(pendingOps.toList()))
    pendingOps.clear()
  }
}
