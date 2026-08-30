package co.typie.ext

import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.imeAnimationTarget
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal actual fun rememberTrustedImeInsets(): WindowInsets {
  val rawImeInsets = WindowInsets.ime
  val animationTargetInsets = WindowInsets.imeAnimationTarget
  return remember(rawImeInsets, animationTargetInsets) {
    trustedImeInsets(
      rawImeInsets = rawImeInsets,
      settledBottom = { density -> animationTargetInsets.getBottom(density).takeIf { it > 0 } },
    )
  }
}
