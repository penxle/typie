package co.typie.overlay

import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import androidx.compose.runtime.staticCompositionLocalOf
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.Duration.Companion.seconds

val LocalToast = staticCompositionLocalOf<Toast> {
  error("No Toast provided")
}

enum class ToastType { Success, Error, Notification, Loading }

data class ToastState(
  val id: Long,
  val type: ToastType,
  val message: String,
  val duration: Duration,
)

class Toast {
  private var nextId = 0L
  private val _state = MutableStateFlow<ToastState?>(null)
  val state: StateFlow<ToastState?> = _state.asStateFlow()
  var bottomInset: Dp by mutableStateOf(0.dp)

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

  suspend fun <T> withLoading(
    message: String,
    errorMessage: String = "오류가 발생했습니다",
    block: suspend LoadingToastScope.() -> T,
  ): T {
    val scope = LoadingToastScope()
    val id = nextId++
    _state.value = ToastState(id, ToastType.Loading, message, Duration.ZERO)
    try {
      val result = scope.block()
      val msg = scope.successMessage ?: message
      if (_state.value?.id == id) {
        _state.value = ToastState(id, ToastType.Success, msg, adaptiveDuration(2.seconds, msg))
      }
      return result
    } catch (e: ToastFailureException) {
      val msg = e.toastMessage
      if (_state.value?.id == id) {
        _state.value = ToastState(id, ToastType.Error, msg, adaptiveDuration(2.seconds, msg))
      }
      throw e
    } catch (e: CancellationException) {
      if (_state.value?.id == id) {
        _state.value = null
      }
      throw e
    } catch (e: Throwable) {
      if (_state.value?.id == id) {
        _state.value = ToastState(id, ToastType.Error, errorMessage, adaptiveDuration(2.seconds, errorMessage))
      }
      throw e
    }
  }
}

class LoadingToastScope {
  internal var successMessage: String? = null

  fun success(message: String) {
    successMessage = message
  }

  fun failure(message: String): Nothing {
    throw ToastFailureException(message)
  }
}

internal class ToastFailureException(val toastMessage: String) : CancellationException(toastMessage)

private fun adaptiveDuration(base: Duration, message: String): Duration {
  val extraMs = (message.length - 18).coerceIn(0, 100) * 12
  val extra = extraMs.milliseconds
  val maxExtra = 1200.milliseconds
  return base + if (extra > maxExtra) maxExtra else extra
}
