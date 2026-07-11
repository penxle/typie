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
    val recorder = editor.inputRecorder
    val imeBefore = if (recorder == null) null else editor.ime
    val state =
      editor.syncWithBringIntoView(bringIntoViewRequests) {
        for (message in messages) {
          enqueue(message)
        }
        beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
      }
    recorder?.record { seq, t ->
      RecordedInputEntry.Dispatch(
        seq = seq,
        t = t,
        messages = messages,
        imeBefore = imeBefore,
        imeAfter = state?.ime,
      )
    }
  }

  private fun recordCall(method: String, args: String) {
    editor.inputRecorder?.record { seq, t ->
      RecordedInputEntry.ImeCall(seq = seq, t = t, method = method, args = args)
    }
  }

  private fun recordRead(method: String, args: String, result: String?) {
    editor.inputRecorder?.record { seq, t ->
      RecordedInputEntry.ImeRead(seq = seq, t = t, method = method, args = args, result = result)
    }
  }

  override fun getTextBeforeCursor(n: Int, flags: Int): CharSequence? {
    val result =
      if (n < 0) {
        null
      } else {
        editor.ime(n, 0)?.let { ctx ->
          ctx.text.substring(0, ctx.selection.start - ctx.windowStart)
        }
      }
    recordRead("getTextBeforeCursor", "n=$n, flags=$flags", result)
    return result
  }

  override fun getTextAfterCursor(n: Int, flags: Int): CharSequence? {
    val result =
      if (n < 0) {
        null
      } else {
        editor.ime(0, n)?.let { ctx -> ctx.text.substring(ctx.selection.end - ctx.windowStart) }
      }
    recordRead("getTextAfterCursor", "n=$n, flags=$flags", result)
    return result
  }

  override fun getSelectedText(flags: Int): CharSequence? {
    val result =
      editor.ime(0, 0)?.let { ctx ->
        val start = ctx.selection.start - ctx.windowStart
        val end = ctx.selection.end - ctx.windowStart
        ctx.text.substring(start, end).ifEmpty { null }
      }
    recordRead("getSelectedText", "flags=$flags", result)
    return result
  }

  @RequiresApi(31)
  override fun getSurroundingText(
    beforeLength: Int,
    afterLength: Int,
    flags: Int,
  ): SurroundingText? {
    val args = "before=$beforeLength, after=$afterLength, flags=$flags"
    if (beforeLength < 0 || afterLength < 0) {
      recordRead("getSurroundingText", args, null)
      return null
    }
    val ctx = editor.ime(beforeLength, afterLength)
    if (ctx == null) {
      recordRead("getSurroundingText", args, null)
      return null
    }
    val selStart = ctx.selection.start - ctx.windowStart
    val selEnd = ctx.selection.end - ctx.windowStart
    recordRead(
      "getSurroundingText",
      args,
      "text=${ctx.text}, selStart=$selStart, selEnd=$selEnd, offset=${ctx.windowStart}",
    )
    return SurroundingText(ctx.text, selStart, selEnd, ctx.windowStart)
  }

  override fun getCursorCapsMode(reqModes: Int): Int = 0

  override fun getExtractedText(request: ExtractedTextRequest?, flags: Int): ExtractedText? = null

  override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
    val value = text?.toString() ?: return false
    recordCall("commitText", "text=$value, newCursorPosition=$newCursorPosition")
    if (value == "\n") {
      batch.enqueue(Message.Key(FfiKeyEvent(Key.Enter)))
    } else {
      batch.commitText(value)
    }
    return true
  }

  override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
    val value = text?.toString() ?: return false
    recordCall("setComposingText", "text=$value, newCursorPosition=$newCursorPosition")
    batch.enqueue(FlatImeOp.Compose(value))
    return true
  }

  override fun setComposingRegion(start: Int, end: Int): Boolean {
    recordCall("setComposingRegion", "start=$start, end=$end")
    batch.enqueue(FlatImeOp.SetComposition(start, end))
    return true
  }

  override fun finishComposingText(): Boolean {
    recordCall("finishComposingText", "")
    batch.finishComposingText(hasActiveComposition = editor.ime?.composing != null)
    return true
  }

  override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
    recordCall("deleteSurroundingText", "before=$beforeLength, after=$afterLength")
    batch.enqueue(FlatImeOp.DeleteSurroundingUtf16(beforeLength, afterLength))
    return true
  }

  override fun deleteSurroundingTextInCodePoints(beforeLength: Int, afterLength: Int): Boolean {
    recordCall("deleteSurroundingTextInCodePoints", "before=$beforeLength, after=$afterLength")
    batch.enqueue(FlatImeOp.DeleteSurrounding(beforeLength, afterLength))
    return true
  }

  override fun setSelection(start: Int, end: Int): Boolean {
    recordCall("setSelection", "start=$start, end=$end")
    batch.enqueue(FlatImeOp.SetSelection(start, end))
    return true
  }

  override fun sendKeyEvent(event: KeyEvent?): Boolean {
    if (event == null) return false
    recordCall("sendKeyEvent", "keyCode=${event.keyCode}, action=${event.action}")
    val key =
      when (event.keyCode) {
        KeyEvent.KEYCODE_DEL -> Key.Backspace
        KeyEvent.KEYCODE_FORWARD_DEL -> Key.Delete
        KeyEvent.KEYCODE_ENTER -> Key.Enter
        KeyEvent.KEYCODE_TAB -> Key.Tab
        KeyEvent.KEYCODE_ESCAPE -> Key.Escape
        else -> return view.dispatchKeyEvent(event)
      }
    if (event.action != KeyEvent.ACTION_DOWN) return false
    batch.enqueue(Message.Key(FfiKeyEvent(key)))
    return true
  }

  override fun performEditorAction(editorAction: Int): Boolean = false

  override fun performContextMenuAction(id: Int): Boolean = false

  override fun beginBatchEdit(): Boolean {
    recordCall("beginBatchEdit", "")
    return batch.beginBatchEdit()
  }

  override fun endBatchEdit(): Boolean {
    recordCall("endBatchEdit", "")
    return batch.endBatchEdit()
  }

  override fun clearMetaKeyStates(states: Int): Boolean = false

  override fun reportFullscreenMode(enabled: Boolean): Boolean = false

  override fun performPrivateCommand(action: String?, data: Bundle?): Boolean = false

  override fun requestCursorUpdates(cursorUpdateMode: Int): Boolean = false

  override fun getHandler() = null

  override fun closeConnection() {
    recordCall("closeConnection", "")
    batch.closeConnection(hasActiveComposition = editor.ime?.composing != null)
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

  fun finishComposingText(hasActiveComposition: Boolean) {
    pendingOps.add(
      if (hasActiveComposition || hasPendingCompositionUpdate()) {
        FlatImeOp.CommitAsIs
      } else {
        FlatImeOp.ClearComposition
      }
    )
    flushIfReady()
  }

  fun closeConnection(hasActiveComposition: Boolean) {
    batchLevel = 0
    if ((hasActiveComposition || hasPendingCompositionUpdate()) && !hasPendingCommitAsIs()) {
      pendingOps.add(FlatImeOp.CommitAsIs)
    } else if (!hasPendingCommitAsIs()) {
      pendingOps.add(FlatImeOp.ClearComposition)
    }
    flush()
  }

  fun commitText(text: String) {
    pendingOps.add(FlatImeOp.Compose(text))
    pendingOps.add(FlatImeOp.CommitAsIs)
    flushIfReady()
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

  private fun hasPendingCompositionUpdate(): Boolean =
    pendingOps.any { it.startsOrUpdatesComposition() } ||
      pendingMessages.any { message ->
        message is Message.TextInput && message.ops.any { it.startsOrUpdatesComposition() }
      }

  private fun hasPendingCommitAsIs(): Boolean =
    pendingOps.any { it == FlatImeOp.CommitAsIs } ||
      pendingMessages.any { message ->
        message is Message.TextInput && message.ops.any { it == FlatImeOp.CommitAsIs }
      }

  private fun FlatImeOp.startsOrUpdatesComposition(): Boolean =
    when (this) {
      is FlatImeOp.Compose,
      is FlatImeOp.SetComposition -> true
      else -> false
    }
}
