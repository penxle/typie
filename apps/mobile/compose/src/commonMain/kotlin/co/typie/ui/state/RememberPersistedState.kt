package co.typie.ui.state

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState as foundationRememberLazyListState
import androidx.compose.foundation.pager.PagerState
import androidx.compose.foundation.pager.rememberPagerState as foundationRememberPagerState
import androidx.compose.foundation.rememberScrollState as foundationRememberScrollState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.currentCompositeKeyHashCode
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key as compositionKey
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

private data class LazyListPosition(val index: Int, val offset: Int)

@Composable
private inline fun <S, reified T> rememberPersistedState(
  key: String?,
  initial: T,
  factory: @Composable (T) -> S,
  crossinline read: (S) -> T,
): S {
  val resolvedKey = key ?: currentCompositeKeyHashCode.toString(36)
  val holder = viewModel<PositionHolder<T>>(key = resolvedKey) { PositionHolder(initial) }
  val restoredInitial = remember(resolvedKey) { holder.position }
  val state = compositionKey(resolvedKey) { factory(restoredInitial) }

  LaunchedEffect(state, holder) { snapshotFlow { read(state) }.collect { holder.position = it } }

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
fun rememberLazyListState(
  key: String? = null,
  initialFirstVisibleItemIndex: Int = 0,
  initialFirstVisibleItemScrollOffset: Int = 0,
): LazyListState =
  rememberPersistedState(
    key = key,
    initial =
      LazyListPosition(
        index = initialFirstVisibleItemIndex,
        offset = initialFirstVisibleItemScrollOffset,
      ),
    factory = {
      foundationRememberLazyListState(
        initialFirstVisibleItemIndex = it.index,
        initialFirstVisibleItemScrollOffset = it.offset,
      )
    },
    read = {
      LazyListPosition(index = it.firstVisibleItemIndex, offset = it.firstVisibleItemScrollOffset)
    },
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
