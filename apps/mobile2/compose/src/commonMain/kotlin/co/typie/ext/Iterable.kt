package co.typie.ext

import androidx.compose.runtime.Composable

@Composable
inline fun <T> Iterable<T>.separated(
  separator: @Composable () -> Unit,
  content: @Composable (T) -> Unit,
) {
  var first = true
  for (item in this) {
    if (!first) separator()
    content(item)
    first = false
  }
}
