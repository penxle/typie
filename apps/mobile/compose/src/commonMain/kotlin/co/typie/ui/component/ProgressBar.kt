package co.typie.ui.component

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import co.typie.domain.goal.GoalColorState
import co.typie.ui.theme.AppShapes

@Composable
fun ProgressBar(progress: Float, state: GoalColorState, modifier: Modifier = Modifier) {
  val trackColor = progressTrackColor()
  val fillColor = progressFillColor(state)
  val fillFraction = progressFillFraction(progress, state)

  Box(
    modifier
      .fillMaxWidth()
      .height(BarHeight)
      .clip(AppShapes.rounded(AppShapes.full))
      .background(trackColor)
  ) {
    Box(
      Modifier.fillMaxHeight()
        .fillMaxWidth(fillFraction)
        .clip(AppShapes.rounded(AppShapes.full))
        .background(fillColor)
    )
  }
}

private val BarHeight = 6.dp
