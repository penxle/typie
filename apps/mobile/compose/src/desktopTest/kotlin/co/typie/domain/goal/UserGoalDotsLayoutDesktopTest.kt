package co.typie.domain.goal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.runComposeUiTest
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals

private const val ProbeItemCount = 40
private val ProbeItemSize = 10.dp
private val ProbeGap = 3.dp

private data class WrapProbe(val density: Float, val widthPx: Int)

@OptIn(ExperimentalTestApi::class)
class UserGoalDotsLayoutDesktopTest {
  @Test
  fun flowRowWrapCountMatchesDotsPerRow() = runComposeUiTest {
    val probes =
      listOf(
        WrapProbe(density = 2.75f, widthPx = 1080),
        WrapProbe(density = 2.75f, widthPx = 992),
        WrapProbe(density = 2.75f, widthPx = 900),
        WrapProbe(density = 1f, widthPx = 328),
        WrapProbe(density = 3.75f, widthPx = 1470),
      )
    val positions = mutableMapOf<Pair<Int, Int>, Offset>()

    setContent {
      Column {
        probes.forEachIndexed { probeIndex, probe ->
          CompositionLocalProvider(LocalDensity provides Density(probe.density)) {
            Layout(
              content = {
                FlowRow(
                  horizontalArrangement = Arrangement.spacedBy(ProbeGap),
                  verticalArrangement = Arrangement.spacedBy(ProbeGap),
                ) {
                  repeat(ProbeItemCount) { itemIndex ->
                    Box(
                      modifier =
                        Modifier.size(ProbeItemSize).onGloballyPositioned { coordinates ->
                          positions[probeIndex to itemIndex] = coordinates.positionInRoot()
                        }
                    )
                  }
                }
              }
            ) { measurables, _ ->
              val placeable = measurables.first().measure(Constraints.fixedWidth(probe.widthPx))
              layout(placeable.width, placeable.height) { placeable.place(0, 0) }
            }
          }
        }
      }
    }
    waitForIdle()

    val measurements = probes.mapIndexed { probeIndex, probe ->
      val items = List(ProbeItemCount) { positions.getValue(probeIndex to it) }
      val firstRowY = items.minOf { it.y }
      val firstRow = items.filter { it.y == firstRowY }
      val metrics = dotMetrics(Density(probe.density))

      println(
        "[flowrow-probe] density=${probe.density} width=${probe.widthPx} " +
          "renderedFirstRow=${firstRow.size} dotsPerRow=${dotsPerRow(metrics, probe.widthPx)} " +
          "renderedStride=${(firstRow.getOrNull(1)?.x ?: 0f) - firstRow[0].x} " +
          "metricsStride=${metrics.stridePx} metricsDot=${metrics.dotPx} " +
          "metricsWrapGap=${metrics.wrapGapPx}"
      )

      Triple(probe, firstRow, metrics)
    }

    measurements.forEach { (probe, firstRow, metrics) ->
      assertEquals(
        firstRow.size,
        dotsPerRow(metrics, probe.widthPx),
        "wrap count mismatch at density=${probe.density} width=${probe.widthPx}",
      )

      assertEquals(
        metrics.stridePx.toFloat(),
        firstRow[1].x - firstRow[0].x,
        absoluteTolerance = 0.001f,
        message = "placement stride mismatch at density=${probe.density} width=${probe.widthPx}",
      )
    }
  }
}
