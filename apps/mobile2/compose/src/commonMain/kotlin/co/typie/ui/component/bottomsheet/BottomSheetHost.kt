package co.typie.ui.component.bottomsheet

import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import kotlin.coroutines.resume
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.suspendCancellableCoroutine

val LocalBottomSheetHost =
  staticCompositionLocalOf<BottomSheetHostState> { error("No BottomSheetHostState provided") }

class BottomSheetEntry<T>(
  val content: @Composable BottomSheetScope<T>.() -> Unit,
  private val continuation: kotlinx.coroutines.CancellableContinuation<T>,
) {
  fun resume(result: T) {
    if (continuation.isActive) continuation.resume(result)
  }

  fun cancel() {
    if (continuation.isActive) continuation.cancel(CancellationException("Bottom sheet dismissed"))
  }
}

class BottomSheetHostState {
  private val _entries = mutableStateListOf<BottomSheetEntry<*>>()
  val entries: List<BottomSheetEntry<*>>
    get() = _entries

  suspend fun <T> show(content: @Composable BottomSheetScope<T>.() -> Unit): T {
    var entryRef: BottomSheetEntry<T>? = null
    try {
      return suspendCancellableCoroutine { continuation ->
        val entry = BottomSheetEntry(content, continuation)
        entryRef = entry
        _entries.add(entry)
      }
    } finally {
      entryRef?.let { _entries.remove(it) }
    }
  }
}

@Composable
fun rememberBottomSheetHostState(): BottomSheetHostState {
  return remember { BottomSheetHostState() }
}
