package co.typie.overlay

import androidx.compose.runtime.staticCompositionLocalOf
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

val LocalLoader = staticCompositionLocalOf<Loader> { error("No Loader provided") }

class Loader {
  private val _loading = MutableStateFlow(false)
  val loading: StateFlow<Boolean> = _loading.asStateFlow()

  suspend fun <T> runWith(block: suspend () -> T): T {
    _loading.value = true
    try {
      return block()
    } finally {
      _loading.value = false
    }
  }
}
