package co.typie.ext

import androidx.compose.foundation.ScrollState
import androidx.compose.ui.Modifier
import androidx.compose.foundation.horizontalScroll as foundationHorizontalScroll
import androidx.compose.foundation.verticalScroll as foundationVerticalScroll

actual fun Modifier.verticalScroll(state: ScrollState): Modifier =
  foundationVerticalScroll(state)

actual fun Modifier.horizontalScroll(state: ScrollState): Modifier =
  foundationHorizontalScroll(state)

actual fun Modifier.overscroll(): Modifier = this
