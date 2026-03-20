package co.typie.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.ui.Modifier

fun Modifier.clickable(onClick: () -> Unit): Modifier =
  clickable(
    interactionSource = MutableInteractionSource(),
    indication = null,
    onClick = onClick,
  )
