package co.typie.ext

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.ui.Modifier
import androidx.compose.foundation.clickable as foundationClickable

expect fun Modifier.verticalScroll(state: ScrollState): Modifier
expect fun Modifier.horizontalScroll(state: ScrollState): Modifier
expect fun Modifier.overscroll(): Modifier

fun Modifier.clickable(onClick: () -> Unit): Modifier =
  foundationClickable(
    interactionSource = MutableInteractionSource(),
    indication = null,
    onClick = onClick,
  )
