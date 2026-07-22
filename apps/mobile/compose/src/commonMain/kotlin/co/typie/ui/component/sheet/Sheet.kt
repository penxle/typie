package co.typie.ui.component.sheet

import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

val LocalSheet = compositionLocalOf<Sheet> { error("No Sheet provided") }

class Sheet {
  internal val entries = mutableStateListOf<SheetEntry<*>>()

  val acceptsInput: Boolean
    get() = entries.any { it.acceptsInput }

  suspend fun <R> present(
    stops: List<SheetStop> = emptyList(),
    stopPolicy: SheetStop.Policy = SheetStop.Policy.KeepAll,
    content:
      @Composable
      context(SheetScope<R>)
      () -> Unit,
  ): R? = suspendCancellableCoroutine { continuation ->
    val entry =
      SheetEntry(
        stops = stops,
        stopPolicy = stopPolicy,
        content = content,
        onResult = { result -> if (continuation.isActive) continuation.resume(result) },
      )
    entries.add(entry)
  }

  internal fun resolveEntry(entry: SheetEntry<*>, result: Any?) {
    entries.remove(entry)
    @Suppress("UNCHECKED_CAST") (entry as SheetEntry<Any?>).onResult(result)
  }

  internal fun stopEntryAcceptingInput(entry: SheetEntry<*>) {
    entry.acceptsInput = false
  }

  internal fun startEntryAcceptingInput(entry: SheetEntry<*>) {
    entry.acceptsInput = true
  }
}

class SheetEntry<R>(
  val stops: List<SheetStop>,
  val stopPolicy: SheetStop.Policy,
  val content:
    @Composable
    context(SheetScope<R>)
    () -> Unit,
  val onResult: (R?) -> Unit,
) {
  var acceptsInput by mutableStateOf(true)
    internal set
}
