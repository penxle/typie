package co.typie.ext

import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.Dp

@Composable internal expect fun rememberTrustedImeBottomInset(): Dp

internal fun trustedImeBottomInset(rawImeBottom: Dp, settledImeBottom: Dp?): Dp =
  if (settledImeBottom != null && rawImeBottom > settledImeBottom) {
    settledImeBottom
  } else {
    rawImeBottom
  }
