package co.typie.ui.component.dialog

import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.mutableStateListOf
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

val LocalDialog = compositionLocalOf<Dialog> { error("No Dialog provided") }

class Dialog {
  internal val queue = mutableStateListOf<DialogEntry<*>>()

  val current: DialogEntry<*>?
    get() = queue.firstOrNull()

  suspend fun <R> present(
    dismissible: Boolean = true,
    content: @Composable context(DialogScope<R>) () -> Unit,
  ): DialogResult<R> = suspendCancellableCoroutine { continuation ->
    val entry =
      DialogEntry(
        dismissible = dismissible,
        content = content,
        onResult = { result -> continuation.resume(result) },
      )
    continuation.invokeOnCancellation { queue.remove(entry) }
    queue.add(entry)
  }

  internal fun resolveCurrentEntry(result: DialogResult<*>) {
    val entry = queue.firstOrNull() ?: return
    queue.removeAt(0)
    @Suppress("UNCHECKED_CAST") (entry as DialogEntry<Any?>).onResult(result)
  }
}

class DialogEntry<R>(
  val dismissible: Boolean,
  val content: @Composable context(DialogScope<R>) () -> Unit,
  val onResult: (DialogResult<R>) -> Unit,
)
