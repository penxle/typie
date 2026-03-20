package co.typie.overlay

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.koin.core.annotation.Single

@Single
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
