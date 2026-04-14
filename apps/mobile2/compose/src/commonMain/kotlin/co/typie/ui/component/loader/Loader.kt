package co.typie.ui.component.loader

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf

val LocalLoader = staticCompositionLocalOf<Loader> { error("No Loader provided") }

class Loader {
  var loading by mutableStateOf(false)
    private set

  suspend fun <T> runWith(block: suspend () -> T): T {
    loading = true
    try {
      return block()
    } finally {
      loading = false
    }
  }
}
