package co.typie.ext

import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.calculateEndPadding
import androidx.compose.foundation.layout.calculateStartPadding
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalLayoutDirection

@Composable
operator fun PaddingValues.plus(other: PaddingValues): PaddingValues {
  val layoutDirection = LocalLayoutDirection.current
  return PaddingValues(
    start = calculateStartPadding(layoutDirection) + other.calculateStartPadding(layoutDirection),
    top = calculateTopPadding() + other.calculateTopPadding(),
    end = calculateEndPadding(layoutDirection) + other.calculateEndPadding(layoutDirection),
    bottom = calculateBottomPadding() + other.calculateBottomPadding(),
  )
}

fun PaddingValues.onlyTop(): PaddingValues = PaddingValues(top = calculateTopPadding())

@Composable
fun PaddingValues.excludeBottom(): PaddingValues {
  val layoutDirection = LocalLayoutDirection.current
  return PaddingValues(
    start = calculateStartPadding(layoutDirection),
    top = calculateTopPadding(),
    end = calculateEndPadding(layoutDirection),
  )
}

@Composable
fun PaddingValues.excludeTop(): PaddingValues {
  val layoutDirection = LocalLayoutDirection.current
  return PaddingValues(
    start = calculateStartPadding(layoutDirection),
    end = calculateEndPadding(layoutDirection),
    bottom = calculateBottomPadding(),
  )
}
