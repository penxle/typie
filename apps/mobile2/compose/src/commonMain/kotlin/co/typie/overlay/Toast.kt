package co.typie.overlay

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.koin.core.annotation.Single
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds

enum class ToastType { Success, Error, Notification }

data class ToastState(
  val id: Long,
  val type: ToastType,
  val message: String,
  val duration: Duration,
)

@Single
class Toast {
  private var nextId = 0L
  private val _state = MutableStateFlow<ToastState?>(null)
  val state: StateFlow<ToastState?> = _state.asStateFlow()

  fun show(
    type: ToastType,
    message: String,
    duration: Duration = 2.seconds,
  ) {
    _state.value = ToastState(nextId++, type, message, adaptiveDuration(duration, message))
  }

  fun dismiss() {
    _state.value = null
  }
}

private fun adaptiveDuration(base: Duration, message: String): Duration {
  val extraMs = (message.length - 18).coerceIn(0, 100) * 12
  val extra = extraMs.milliseconds
  val maxExtra = 1200.milliseconds
  return base + if (extra > maxExtra) maxExtra else extra
}
