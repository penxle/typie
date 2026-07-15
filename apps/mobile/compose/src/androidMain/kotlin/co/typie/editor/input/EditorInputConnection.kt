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

// Reads are bounded by the ime window itself, and keyboards derive absolute
// positions from read lengths (e.g. getTextBeforeCursor(Int.MAX_VALUE)), so
// reads must never be capped below the window — the returned prefix length is
// the window-relative cursor position. This clamp only keeps huge requested
// lengths from overflowing the Int arithmetic in the window trim.
private const val IME_READ_OVERFLOW_GUARD = 1 shl 24

// Samsung keyboards use getExtractedText as the source of truth for their
// internal editing-state model; monitoring spans connections within a session.
internal class ImeExtractMonitor {
  var token: Int? = null
}

internal class EditorInputConnection(
  private val editor: Editor,
  private val view: View,
  private val bringIntoViewRequests: EditorBringIntoViewRequests,
  private val extractMonitor: ImeExtractMonitor,
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
        editor.tickIme?.trimmedTo(n.coerceAtMost(IME_READ_OVERFLOW_GUARD), 0)?.let { ctx ->
          ctx.text.substring(
            0,
            ctx.text.utf16IndexAtCodePointOffset(ctx.selection.start - ctx.windowStart),
          )
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
        editor.tickIme?.trimmedTo(0, n.coerceAtMost(IME_READ_OVERFLOW_GUARD))?.let { ctx ->
          ctx.text.substring(
            ctx.text.utf16IndexAtCodePointOffset(ctx.selection.end - ctx.windowStart)
          )
        }
      }
    recordRead("getTextAfterCursor", "n=$n, flags=$flags", result)
    return result
  }

  override fun getSelectedText(flags: Int): CharSequence? {
    val result =
      editor.tickIme?.trimmedTo(0, 0)?.let { ctx ->
        val start = ctx.text.utf16IndexAtCodePointOffset(ctx.selection.start - ctx.windowStart)
        val end = ctx.text.utf16IndexAtCodePointOffset(ctx.selection.end - ctx.windowStart)
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
    val full = editor.tickIme
    val ctx =
      full?.trimmedTo(
        beforeLength.coerceAtMost(IME_READ_OVERFLOW_GUARD),
        afterLength.coerceAtMost(IME_READ_OVERFLOW_GUARD),
      )
    if (full == null || ctx == null) {
      recordRead("getSurroundingText", args, null)
      return null
    }
    val selStart = ctx.text.utf16IndexAtCodePointOffset(ctx.selection.start - ctx.windowStart)
    val selEnd = ctx.text.utf16IndexAtCodePointOffset(ctx.selection.end - ctx.windowStart)
    // Window-relative world: the offset locates the trimmed text within the
    // window presented as the whole document.
    val offset = full.windowUtf16Offset(ctx.windowStart)
    recordRead(
      "getSurroundingText",
      args,
      "text=${ctx.text}, selStart=$selStart, selEnd=$selEnd, offset=$offset",
    )
    return SurroundingText(ctx.text, selStart, selEnd, offset)
  }

  override fun getCursorCapsMode(reqModes: Int): Int = 0

  override fun getExtractedText(request: ExtractedTextRequest?, flags: Int): ExtractedText? {
    if (request != null && (flags and InputConnection.GET_EXTRACTED_TEXT_MONITOR) != 0) {
      extractMonitor.token = request.token
    }
    val extract = editor.tickIme?.extract()
    recordRead(
      "getExtractedText",
      "token=${request?.token}, flags=$flags",
      extract?.let { "sel=${it.selectionStart}..${it.selectionEnd}, textLength=${it.text.length}" },
    )
    return extract?.toExtractedText()
  }

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
    when (val decision = resolveComposingRegion(editor.tickIme, start, end)) {
      is ComposingRegionDecision.Set ->
        batch.enqueue(FlatImeOp.SetComposition(decision.start, decision.end))
      ComposingRegionDecision.Clear -> batch.enqueue(FlatImeOp.ClearComposition)
    }
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
    val ctx = editor.tickIme ?: return true
    batch.enqueue(
      FlatImeOp.SetSelection(ctx.projectWindowUtf16Index(start), ctx.projectWindowUtf16Index(end))
    )
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
    extractMonitor.token = null
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

internal fun ImeExtract.toExtractedText(): ExtractedText =
  ExtractedText().also {
    it.text = text
    // Window-relative world: the window is the whole presented document.
    it.startOffset = 0
    it.partialStartOffset = -1
    it.partialEndOffset = -1
    it.selectionStart = selectionStart
    it.selectionEnd = selectionEnd
    it.flags = if (selectionStart != selectionEnd) ExtractedText.FLAG_SELECTING else 0
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
    // The editor has no inline newline: multi-line commits (e.g. keyboard
    // clipboard suggestions) become paragraph splits via the enter key path.
    val segments = text.replace("\r\n", "\n").replace('\r', '\n').split("\n")
    segments.forEachIndexed { index, segment ->
      if (index > 0) {
        flushOpsToPendingMessages()
        pendingMessages.add(Message.Key(FfiKeyEvent(Key.Enter)))
      }
      if (segment.isNotEmpty() || index == 0) {
        pendingOps.add(FlatImeOp.Compose(segment))
        pendingOps.add(FlatImeOp.CommitAsIs)
      }
    }
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
