package co.typie.ui.input

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier

@Composable
internal fun WindowInputTestHost(
  modifier: Modifier = Modifier,
  content: @Composable BoxScope.() -> Unit,
) {
  val state = remember { WindowInputState() }
  CompositionLocalProvider(LocalWindowInputState provides state) {
    Box(modifier.windowInput(state), content = content)
  }
}
