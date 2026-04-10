package co.typie.ext

import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp

fun Int.toDp(density: Density): Dp = with(density) { toDp() }

fun Float.toDp(density: Density): Dp = with(density) { toDp() }

fun Dp.toPx(density: Density): Float = with(density) { toPx() }
