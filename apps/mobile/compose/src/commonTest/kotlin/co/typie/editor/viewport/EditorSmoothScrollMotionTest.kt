package co.typie.editor.viewport

import kotlin.math.abs
import kotlin.math.max
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EditorSmoothScrollMotionTest {
  @Test
  fun `translation preserves velocity and remaining distance`() {
    val motion =
      EditorSmoothScrollMotion.start(position = 100.0, target = 500.0, viewportHeight = 400.0)
    motion.advance(1.0 / 60.0)
    val before = motion.snapshot()

    motion.translate(700.0)

    val after = motion.snapshot()
    assertEquals(before.position + 700.0, after.position, 1e-8)
    assertEquals(before.target + 700.0, after.target, 1e-8)
    assertEquals(before.velocity, after.velocity, 1e-8)
    assertEquals(before.target - before.position, after.target - after.position, 1e-8)
  }

  @Test
  fun `matching elapsed time is frame rate independent`() {
    val at60Hz =
      EditorSmoothScrollMotion.start(position = 0.0, target = 1600.0, viewportHeight = 400.0)
    val at120Hz =
      EditorSmoothScrollMotion.start(position = 0.0, target = 1600.0, viewportHeight = 400.0)

    repeat(30) { at60Hz.advance(1.0 / 60.0) }
    repeat(60) { at120Hz.advance(1.0 / 120.0) }

    assertEquals(at60Hz.snapshot().position, at120Hz.snapshot().position, 1e-8)
    assertEquals(at60Hz.snapshot().velocity, at120Hz.snapshot().velocity, 1e-8)
  }

  @Test
  fun `matches the Web retarget and translation vector`() {
    val motion =
      EditorSmoothScrollMotion.start(position = 0.0, target = 1600.0, viewportHeight = 400.0)
    motion.advance(0.05)
    motion.advance(0.05)
    motion.translate(700.0)
    motion.retarget(target = 2100.0, viewportHeight = 400.0)
    motion.advance(0.05)

    assertEquals(1809.7452631440876, motion.snapshot().position, 0.01)
    assertEquals(4556.356253653357, motion.snapshot().velocity, 0.1)
    assertEquals(2100.0, motion.snapshot().target)
  }

  @Test
  fun `farther destination takes longer and reaches a higher speed`() {
    val short =
      runToCompletion(
        EditorSmoothScrollMotion.start(position = 0.0, target = 100.0, viewportHeight = 400.0)
      )
    val long =
      runToCompletion(
        EditorSmoothScrollMotion.start(position = 0.0, target = 1600.0, viewportHeight = 400.0)
      )

    assertTrue(long.elapsed > short.elapsed)
    assertTrue(long.elapsed < short.elapsed * 4.0)
    assertTrue(long.peakVelocity > short.peakVelocity)
  }

  @Test
  fun `retarget preserves current position and velocity`() {
    val motion =
      EditorSmoothScrollMotion.start(position = 0.0, target = 800.0, viewportHeight = 400.0)
    repeat(8) { motion.advance(1.0 / 60.0) }
    val before = motion.snapshot()

    motion.retarget(target = 1200.0, viewportHeight = 400.0)

    assertEquals(before.position, motion.snapshot().position)
    assertEquals(before.velocity, motion.snapshot().velocity)
    assertEquals(1200.0, motion.snapshot().target)
  }

  @Test
  fun `within-threshold retarget finishes despite remaining velocity`() {
    val motion =
      EditorSmoothScrollMotion.start(position = 0.0, target = 800.0, viewportHeight = 400.0)
    repeat(8) { motion.advance(1.0 / 60.0) }
    val target = motion.snapshot().position + 0.25

    motion.retarget(target = target, viewportHeight = 400.0)

    assertTrue(motion.finished)
    assertEquals(
      EditorSmoothScrollMotion.Snapshot(position = target, velocity = 0.0, target = target),
      motion.snapshot(),
    )
  }

  @Test
  fun `high-speed closer retarget does not overshoot`() {
    val motion =
      EditorSmoothScrollMotion.start(position = 0.0, target = 1600.0, viewportHeight = 400.0)
    repeat(12) { motion.advance(1.0 / 60.0) }
    val before = motion.snapshot()
    val target = before.position + 80.0

    motion.retarget(target = target, viewportHeight = 400.0)
    assertEquals(before.velocity, motion.snapshot().velocity)

    while (!motion.finished) {
      assertTrue(motion.advance(1.0 / 120.0).position <= target)
    }
    assertEquals(
      EditorSmoothScrollMotion.Snapshot(position = target, velocity = 0.0, target = target),
      motion.snapshot(),
    )
  }

  @Test
  fun `bound synchronization removes outward velocity and adopts host position`() {
    val motion =
      EditorSmoothScrollMotion.start(position = 0.0, target = 1600.0, viewportHeight = 400.0)
    repeat(6) { motion.advance(1.0 / 60.0) }

    motion.synchronizeBounds(actualPosition = 600.0, maximumScroll = 600.0, viewportHeight = 400.0)

    assertTrue(motion.finished)
    assertEquals(
      EditorSmoothScrollMotion.Snapshot(position = 600.0, velocity = 0.0, target = 600.0),
      motion.snapshot(),
    )
  }

  @Test
  fun `delayed frame catches up to the full elapsed time`() {
    val delayed =
      EditorSmoothScrollMotion.start(position = 0.0, target = 800.0, viewportHeight = 400.0)
    val uninterrupted =
      EditorSmoothScrollMotion.start(position = 0.0, target = 800.0, viewportHeight = 400.0)

    delayed.advance(0.25)
    repeat(15) { uninterrupted.advance(1.0 / 60.0) }

    assertEquals(uninterrupted.snapshot().position, delayed.snapshot().position, 1e-8)
    assertEquals(uninterrupted.snapshot().velocity, delayed.snapshot().velocity, 1e-8)
  }

  private fun runToCompletion(motion: EditorSmoothScrollMotion): RunResult {
    var elapsed = 0.0
    var peakVelocity = 0.0
    while (!motion.finished && elapsed < 5.0) {
      val state = motion.advance(1.0 / 120.0)
      elapsed += 1.0 / 120.0
      peakVelocity = max(peakVelocity, abs(state.velocity))
    }
    assertTrue(motion.finished)
    return RunResult(elapsed = elapsed, peakVelocity = peakVelocity)
  }

  private data class RunResult(val elapsed: Double, val peakVelocity: Double)
}
