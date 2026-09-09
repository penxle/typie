package co.typie.editor.input

import android.content.Context
import android.graphics.Matrix
import android.graphics.RectF
import android.os.Build
import android.view.View
import android.view.ViewTreeObserver
import android.view.inputmethod.CursorAnchorInfo
import android.view.inputmethod.EditorBoundsInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.text.TextRange
import co.typie.editor.ffi.Ime
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

internal class EditorCursorAnchorInfoController(
  private val view: View,
  private val scope: CoroutineScope,
  private val ime: () -> Ime?,
  private val focusedRectInRoot: () -> Rect?,
  private val firstRectForRangeInRoot: (TextRange) -> Rect?,
  private val textFieldRectInRoot: () -> Rect?,
  private val textClippingRectInRoot: () -> Rect?,
  private val isSessionCurrent: () -> Boolean,
) {
  private var updates: Job? = null
  private var closed = false
  private var pendingImmediate = false
  private var monitor = false
  private var filters = 0

  fun requestUpdates(mode: Int): Boolean {
    if (closed || !scope.isActive || !isSessionCurrent() || mode and SUPPORTED_FLAGS.inv() != 0)
      return false
    pendingImmediate = mode and InputConnection.CURSOR_UPDATE_IMMEDIATE != 0
    monitor = mode and InputConnection.CURSOR_UPDATE_MONITOR != 0
    filters = mode and FILTER_FLAGS
    if (!pendingImmediate && !monitor) {
      updates?.cancel()
      updates = null
      return true
    }
    if (updates == null) {
      updates =
        scope.launch(start = CoroutineStart.UNDISPATCHED) {
          // Report after layout so scrolling, zooming and moving the Android window all use
          // the same coordinates as the frame about to be drawn.
          val listener = ViewTreeObserver.OnPreDrawListener {
            if (pendingImmediate || monitor) update()
            true
          }
          val observer = view.viewTreeObserver
          observer.addOnPreDrawListener(listener)
          try {
            awaitCancellation()
          } finally {
            if (observer.isAlive) observer.removeOnPreDrawListener(listener)
            else view.viewTreeObserver.removeOnPreDrawListener(listener)
          }
        }
    }
    view.invalidate()
    return true
  }

  fun close() {
    closed = true
    updates?.cancel()
    updates = null
  }

  private fun update() {
    if (closed || !isSessionCurrent()) return
    val ctx = ime() ?: return
    val caret = focusedRectInRoot() ?: return
    val clipping = textClippingRectInRoot() ?: return
    val imm =
      view.context.getSystemService(Context.INPUT_METHOD_SERVICE) as? InputMethodManager ?: return
    if (!imm.isActive(view)) return

    val matrix = Matrix()
    view.transformMatrixToGlobal(matrix)
    val builder = CursorAnchorInfo.Builder().setMatrix(matrix)
    builder.setSelectionRange(
      ctx.windowUtf16Offset(ctx.selection.start),
      ctx.windowUtf16Offset(ctx.selection.end),
    )
    if (includes(InputConnection.CURSOR_UPDATE_FILTER_INSERTION_MARKER)) {
      builder.setInsertionMarkerLocation(
        caret.left,
        caret.top,
        caret.bottom,
        caret.bottom,
        caret.visibilityFlags(clipping),
      )
    }
    val composition = ctx.composing
    if (composition != null) {
      val start = ctx.windowUtf16Offset(composition.start)
      val end = ctx.windowUtf16Offset(composition.end)
      builder.setComposingText(start, ctx.text.subSequence(start, end))
      if (includes(InputConnection.CURSOR_UPDATE_FILTER_CHARACTER_BOUNDS)) {
        var index = start
        while (index < end) {
          val next = index + Character.charCount(Character.codePointAt(ctx.text, index))
          val rect = firstRectForRangeInRoot(TextRange(index, next))
          if (rect != null) {
            for (utf16Index in index until next) {
              builder.addCharacterBounds(
                utf16Index,
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
                rect.visibilityFlags(clipping),
              )
            }
          }
          index = next
        }
      }
    }
    if (
      Build.VERSION.SDK_INT >= 33 && includes(InputConnection.CURSOR_UPDATE_FILTER_EDITOR_BOUNDS)
    ) {
      textFieldRectInRoot()?.let { bounds ->
        builder.setEditorBoundsInfo(
          EditorBoundsInfo.Builder()
            .setEditorBounds(RectF(bounds.left, bounds.top, bounds.right, bounds.bottom))
            .setHandwritingBounds(RectF(bounds.left, bounds.top, bounds.right, bounds.bottom))
            .build()
        )
      }
    }
    imm.updateCursorAnchorInfo(view, builder.build())
    pendingImmediate = false
    if (!monitor) {
      updates?.cancel()
      updates = null
    }
  }

  private fun includes(filter: Int): Boolean = filters == 0 || filters and filter != 0

  private fun Rect.visibilityFlags(clipping: Rect): Int {
    var flags = 0
    if (
      right >= clipping.left &&
        left <= clipping.right &&
        bottom >= clipping.top &&
        top <= clipping.bottom
    ) {
      flags = flags or CursorAnchorInfo.FLAG_HAS_VISIBLE_REGION
    }
    if (
      left < clipping.left ||
        right > clipping.right ||
        top < clipping.top ||
        bottom > clipping.bottom
    ) {
      flags = flags or CursorAnchorInfo.FLAG_HAS_INVISIBLE_REGION
    }
    return flags
  }

  private companion object {
    const val FILTER_FLAGS =
      InputConnection.CURSOR_UPDATE_FILTER_INSERTION_MARKER or
        InputConnection.CURSOR_UPDATE_FILTER_CHARACTER_BOUNDS or
        InputConnection.CURSOR_UPDATE_FILTER_EDITOR_BOUNDS
    const val SUPPORTED_FLAGS =
      InputConnection.CURSOR_UPDATE_IMMEDIATE or
        InputConnection.CURSOR_UPDATE_MONITOR or
        FILTER_FLAGS
  }
}
