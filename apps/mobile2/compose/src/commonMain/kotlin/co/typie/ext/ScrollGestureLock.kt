package co.typie.ext

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf

private val DefaultScrollGestureLockState = ScrollGestureLockState()

internal val LocalScrollGestureLockState = staticCompositionLocalOf { DefaultScrollGestureLockState }

@Stable
class ScrollGestureLockState {
  private var lockCount by mutableIntStateOf(0)

  val isLocked: Boolean
    get() = lockCount > 0

  fun acquire(): ScrollGestureLockHandle {
    lockCount += 1
    return ScrollGestureLockHandle(this)
  }

  internal fun release() {
    if (lockCount > 0) {
      lockCount -= 1
    }
  }
}

class ScrollGestureLockHandle internal constructor(
  private val state: ScrollGestureLockState,
) {
  private var released = false

  fun release() {
    if (released) {
      return
    }

    released = true
    state.release()
  }
}

@Composable
internal fun ScrollGestureLockScope(content: @Composable () -> Unit) {
  val lockState = remember { ScrollGestureLockState() }
  CompositionLocalProvider(LocalScrollGestureLockState provides lockState) {
    content()
  }
}
