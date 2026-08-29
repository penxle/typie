package co.typie.ui.skeleton

import androidx.compose.foundation.layout.size
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.MotionDurationScale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.toPixelMap
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.v2.runComposeUiTest
import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

@OptIn(ExperimentalTestApi::class)
class SkeletonDesktopTest {
  @Test
  fun motionScaleStopsAndRestartsTheSkeletonShimmer() {
    val motionScale = TestMotionDurationScale(1f)

    runComposeUiTest(effectContext = motionScale) {
      mainClock.autoAdvance = false
      setContent {
        Skeleton(
          enabled = true,
          colors = SkeletonColors(bone = Color.Red, highlight = Color.Blue),
        ) {
          SkeletonBone(Modifier.size(40.dp).testTag(SkeletonTag), RectangleShape)
        }
      }
      mainClock.advanceTimeByFrame()
      mainClock.advanceTimeByFrame()

      val animatedStart = onNodeWithTag(SkeletonTag).captureToImage().centerColor()
      mainClock.advanceTimeBy(200L)
      val animatedEnd = onNodeWithTag(SkeletonTag).captureToImage().centerColor()
      assertNotEquals(animatedStart, animatedEnd)

      runOnIdle { motionScale.scaleFactor = 0f }
      mainClock.advanceTimeByFrame()
      val reducedStart = onNodeWithTag(SkeletonTag).captureToImage().centerColor()
      mainClock.advanceTimeBy(400L)
      val reducedEnd = onNodeWithTag(SkeletonTag).captureToImage().centerColor()
      assertEquals(reducedStart, reducedEnd)

      runOnIdle { motionScale.scaleFactor = 1f }
      mainClock.advanceTimeByFrame()
      val resumedStart = onNodeWithTag(SkeletonTag).captureToImage().centerColor()
      mainClock.advanceTimeBy(200L)
      val resumedEnd = onNodeWithTag(SkeletonTag).captureToImage().centerColor()
      assertNotEquals(resumedStart, resumedEnd)
    }
  }

  private class TestMotionDurationScale(initialScaleFactor: Float) : MotionDurationScale {
    override var scaleFactor by mutableFloatStateOf(initialScaleFactor)
  }

  private companion object {
    const val SkeletonTag = "skeleton"
  }
}

private fun androidx.compose.ui.graphics.ImageBitmap.centerColor(): Color =
  toPixelMap()[width / 2, height / 2]
