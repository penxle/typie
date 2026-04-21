package co.typie.ui.state

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState as foundationRememberPagerState
import androidx.compose.foundation.rememberScrollState as foundationRememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.currentCompositeKeyHashCode
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.toString
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel

private class PositionHolder<T>(initial: T) : ViewModel() {
  var position by mutableStateOf(initial)
}

@Composable
private inline fun <S, reified T> rememberPersistedState(
  key: String?,
  initial: T,
  factory: @Composable (T) -> S,
  crossinline read: (S) -> T,
): S {
  val resolvedKey = key ?: currentCompositeKeyHashCode.toString(36)
  val holder = viewModel<PositionHolder<T>>(key = resolvedKey) { PositionHolder(initial) }
  val restoredInitial = remember { holder.position }
  val state = factory(restoredInitial)

  LaunchedEffect(state) { snapshotFlow { read(state) }.collect { holder.position = it } }

  return state
}

@Composable
fun rememberScrollState(key: String? = null, initial: Int = 0): ScrollState =
  rememberPersistedState(
    key = key,
    initial = initial,
    factory = { foundationRememberScrollState(initial = it) },
    read = { it.value },
  )

@Composable
fun rememberPagerState(
  key: String? = null,
  initialPage: Int = 0,
  initialPageOffsetFraction: Float = 0f,
  pageCount: () -> Int,
): PagerState =
  rememberPersistedState(
    key = key,
    initial = initialPage,
    factory = {
      foundationRememberPagerState(
        initialPage = it,
        initialPageOffsetFraction = initialPageOffsetFraction,
        pageCount = pageCount,
      )
    },
    read = { it.currentPage },
  )
