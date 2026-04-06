package co.typie.ui.state

import androidx.compose.foundation.ScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.compose.foundation.rememberScrollState as foundationRememberScrollState

private class ScrollPositionHolder(initial: Int) : ViewModel() {
  var position by mutableStateOf(initial)
}

@Composable
fun rememberScrollState(key: String? = null, initial: Int = 0): ScrollState {
  val holder = viewModel(key = key) { ScrollPositionHolder(initial) }
  val restoredInitial = remember { holder.position }
  val scrollState = foundationRememberScrollState(initial = restoredInitial)

  LaunchedEffect(scrollState) {
    snapshotFlow { scrollState.value }
      .collect { holder.position = it }
  }

  return scrollState
}
