package co.typie.ui.state

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

class AsyncAction(
  private val scope: CoroutineScope,
) {
  var running by mutableStateOf(false)
    private set

  fun launch(
    onFailure: (Exception) -> Unit = {},
    block: suspend () -> Unit,
  ) {
    if (running) return

    scope.launch {
      running = true
      try {
        block()
      } catch (e: CancellationException) {
        throw e
      } catch (e: Exception) {
        onFailure(e)
      } finally {
        running = false
      }
    }
  }
}
