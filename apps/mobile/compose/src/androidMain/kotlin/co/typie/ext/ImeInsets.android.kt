package co.typie.ext

import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.imeAnimationTarget
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalLayoutApi::class)
@Composable
internal actual fun rememberTrustedImeBottomInset() =
  with(LocalDensity.current) {
    val rawImeBottom = WindowInsets.ime.getBottom(this).toDp()
    val animationTargetBottom = WindowInsets.imeAnimationTarget.getBottom(this).toDp()
    trustedImeBottomInset(
      rawImeBottom = rawImeBottom,
      settledImeBottom = animationTargetBottom.takeIf { it > 0.dp },
    )
  }
