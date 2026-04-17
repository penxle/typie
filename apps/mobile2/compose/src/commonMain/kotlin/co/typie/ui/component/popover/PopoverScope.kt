package co.typie.ui.component.popover

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

@Stable
class PopoverScope internal constructor(private val onClose: () -> Unit) {
  internal var pressGestureSession: PressGestureSession? by mutableStateOf(null)
    internal set

  var acceptsInput: Boolean by mutableStateOf(true)
    internal set

  fun close() {
    acceptsInput = false
    onClose()
  }
}

context(scope: PopoverScope)
fun close(): Unit = scope.close()
