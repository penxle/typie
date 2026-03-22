package co.typie.ext

import androidx.compose.foundation.ScrollState
import androidx.compose.ui.Modifier
import androidx.compose.foundation.horizontalScroll as foundationHorizontalScroll
import androidx.compose.foundation.verticalScroll as foundationVerticalScroll

actual fun Modifier.verticalScroll(state: ScrollState, enabled: Boolean): Modifier =
  foundationVerticalScroll(state, enabled = enabled)

actual fun Modifier.horizontalScroll(state: ScrollState, enabled: Boolean): Modifier =
  foundationHorizontalScroll(state, enabled = enabled)

actual fun Modifier.overscroll(): Modifier = this
