package co.typie.platform

import androidx.core.graphics.Insets
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class AndroidSoftwareKeyboardPresentationTest {
  @Test
  fun latestPendingProgressIsAppliedAcrossEveryInsetEdgeWhenControlBecomesReady() {
    val driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = {},
        cancelControl = {},
        hideIme = {},
        isImeVisible = { true },
      )
    val control =
      RecordingAndroidImeAnimationControl(
        hiddenStateInsets = Insets.of(2, 4, 6, 8),
        shownStateInsets = Insets.of(10, 20, 30, 40),
        currentAlpha = 0.4f,
      )

    driver.updateHiddenProgress(0.25f)
    driver.updateHiddenProgress(0.75f)
    driver.onReady(control)

    assertEquals(
      listOf(AndroidImeFrame(Insets.of(4, 8, 12, 16), alpha = 0.4f, fraction = 0.25f)),
      control.frames,
    )
  }

  @Test
  fun completedEndpointAnimationFinishesAtTheRequestedEndpointBeforeAcceptingIt() {
    for ((endpoint, shown) in
      listOf(
        SoftwareKeyboardPresentationEndpoint.Shown to true,
        SoftwareKeyboardPresentationEndpoint.Hidden to false,
      )) {
      val driver =
        AndroidSoftwareKeyboardPresentationDriver(
          onInvalidated = {},
          cancelControl = {},
          hideIme = {},
          isImeVisible = { true },
          endpointAnimator = ImmediateAndroidImeEndpointAnimator,
        )
      val control = RecordingAndroidImeAnimationControl()
      var accepted = false
      driver.onReady(control)

      driver.finish(endpoint) {
        assertEquals(listOf(shown), control.finishes)
        accepted = true
      }

      assertTrue(accepted)
    }
  }

  @Test
  fun terminalRequestContinuesFromCurrentProgressBeforeFinishingControl() {
    val endpointAnimator = ManualAndroidImeEndpointAnimator()
    val driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = {},
        cancelControl = {},
        hideIme = {},
        isImeVisible = { true },
        endpointAnimator = endpointAnimator,
      )
    val control =
      RecordingAndroidImeAnimationControl(
        hiddenStateInsets = Insets.NONE,
        shownStateInsets = Insets.of(0, 0, 0, 100),
      )
    var accepted = false
    driver.updateHiddenProgress(0.6f)
    driver.onReady(control)

    driver.finish(SoftwareKeyboardPresentationEndpoint.Hidden) { accepted = true }

    assertEquals(0.6f, endpointAnimator.fromHiddenProgress)
    assertEquals(1f, endpointAnimator.toHiddenProgress)
    assertTrue(control.finishes.isEmpty())
    assertFalse(accepted)

    endpointAnimator.updateProgress(0.8f)

    assertEquals(Insets.of(0, 0, 0, 20), control.frames.last().insets)
    assertTrue(control.finishes.isEmpty())
    assertFalse(accepted)

    endpointAnimator.finish()

    assertEquals(Insets.NONE, control.frames.last().insets)
    assertEquals(listOf(false), control.finishes)
    assertTrue(accepted)
  }

  @Test
  fun terminalRequestBeforeReadinessAnimatesFromTheLatestProgressWhenControlArrives() {
    for ((endpoint, shown) in
      listOf(
        SoftwareKeyboardPresentationEndpoint.Shown to true,
        SoftwareKeyboardPresentationEndpoint.Hidden to false,
      )) {
      val endpointAnimator = ManualAndroidImeEndpointAnimator()
      val driver =
        AndroidSoftwareKeyboardPresentationDriver(
          onInvalidated = {},
          cancelControl = {},
          hideIme = {},
          isImeVisible = { true },
          endpointAnimator = endpointAnimator,
        )
      val control = RecordingAndroidImeAnimationControl()
      var accepted = false
      driver.updateHiddenProgress(0.6f)

      driver.finish(endpoint) { accepted = true }
      driver.onReady(control)

      assertEquals(0.6f, endpointAnimator.fromHiddenProgress)
      assertEquals(if (shown) 0f else 1f, endpointAnimator.toHiddenProgress)
      assertTrue(control.finishes.isEmpty())
      assertFalse(accepted)

      endpointAnimator.finish()

      assertEquals(listOf(shown), control.finishes)
      assertTrue(accepted)
    }
  }

  @Test
  fun cancellationBeforeReadinessInvalidatesOnceAndIgnoresLateReadiness() {
    var invalidations = 0
    val driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { invalidations += 1 },
        cancelControl = {},
        hideIme = {},
        isImeVisible = { true },
      )
    val lateControl = RecordingAndroidImeAnimationControl()

    driver.onCancelled()
    driver.onCancelled()
    driver.updateHiddenProgress(0.8f)
    driver.onReady(lateControl)

    assertEquals(1, invalidations)
    assertTrue(lateControl.frames.isEmpty())
    assertTrue(lateControl.finishes.isEmpty())
  }

  @Test
  fun cancellationAfterHiddenRequestFallsBackToHideBeforeAcceptingIt() {
    val events = mutableListOf<String>()
    var invalidations = 0
    val driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { invalidations += 1 },
        cancelControl = {},
        hideIme = { events += "hide" },
        isImeVisible = { true },
        endpointAnimator = ImmediateAndroidImeEndpointAnimator,
      )

    driver.finish(SoftwareKeyboardPresentationEndpoint.Hidden) { events += "accepted" }
    driver.onCancelled()
    driver.onCancelled()

    assertEquals(listOf("hide", "accepted"), events)
    assertEquals(0, invalidations)
  }

  @Test
  fun cancellationAfterShownRequestIsAcceptedOnlyIfTheImeIsVisible() {
    for (imeVisible in listOf(true, false)) {
      var accepted = false
      var invalidations = 0
      val driver =
        AndroidSoftwareKeyboardPresentationDriver(
          onInvalidated = { invalidations += 1 },
          cancelControl = {},
          hideIme = {},
          isImeVisible = { imeVisible },
        )

      driver.finish(SoftwareKeyboardPresentationEndpoint.Shown) { accepted = true }
      driver.onCancelled()

      assertEquals(imeVisible, accepted)
      assertEquals(if (imeVisible) 0 else 1, invalidations)
    }
  }

  @Test
  fun synchronousCancellationDuringProgressCannotReviveTheDriver() {
    var invalidations = 0
    lateinit var driver: AndroidSoftwareKeyboardPresentationDriver
    val control = RecordingAndroidImeAnimationControl()
    driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { invalidations += 1 },
        cancelControl = {},
        hideIme = {},
        isImeVisible = { true },
      )
    driver.onReady(control)
    control.onSetInsetsAndAlpha = driver::onCancelled

    driver.updateHiddenProgress(0.5f)
    driver.updateHiddenProgress(0.75f)

    assertEquals(1, invalidations)
    assertEquals(2, control.frames.size)
  }

  @Test
  fun synchronousCancellationDuringFinishAcceptsTheTerminalRequestOnce() {
    val events = mutableListOf<String>()
    lateinit var driver: AndroidSoftwareKeyboardPresentationDriver
    val control = RecordingAndroidImeAnimationControl()
    driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { events += "invalidated" },
        cancelControl = {},
        hideIme = { events += "hide" },
        isImeVisible = { true },
        endpointAnimator = ImmediateAndroidImeEndpointAnimator,
      )
    driver.onReady(control)
    control.onFinish = {
      events += "finish"
      driver.onCancelled()
    }

    driver.finish(SoftwareKeyboardPresentationEndpoint.Hidden) { events += "accepted" }
    driver.onCancelled()

    assertEquals(listOf("finish", "hide", "accepted"), events)
  }

  @Test
  fun disposalCancelsControlOnceAndMakesSynchronousAndLateCallbacksHarmless() {
    var cancellations = 0
    var invalidations = 0
    lateinit var driver: AndroidSoftwareKeyboardPresentationDriver
    val lateControl = RecordingAndroidImeAnimationControl().apply { isReady = false }
    driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { invalidations += 1 },
        cancelControl = {
          cancellations += 1
          driver.onCancelled()
        },
        hideIme = {},
        isImeVisible = { true },
      )

    driver.dispose()
    driver.dispose()
    driver.onReady(lateControl)
    driver.updateHiddenProgress(0.5f)

    assertEquals(1, cancellations)
    assertEquals(0, invalidations)
    assertTrue(lateControl.frames.isEmpty())
  }

  @Test
  fun successfulPlatformFinishAcceptsTheTerminalRequestOnce() {
    var acceptances = 0
    var invalidations = 0
    lateinit var driver: AndroidSoftwareKeyboardPresentationDriver
    val control = RecordingAndroidImeAnimationControl()
    driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { invalidations += 1 },
        cancelControl = {},
        hideIme = {},
        isImeVisible = { true },
        endpointAnimator = ImmediateAndroidImeEndpointAnimator,
      )
    control.onFinish = driver::onFinished
    driver.finish(SoftwareKeyboardPresentationEndpoint.Shown) { acceptances += 1 }

    driver.onReady(control)
    driver.onFinished()

    assertEquals(1, acceptances)
    assertEquals(0, invalidations)
  }

  @Test
  fun unexpectedPlatformFinishInvalidatesTheActiveDriverOnce() {
    var invalidations = 0
    val driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { invalidations += 1 },
        cancelControl = {},
        hideIme = {},
        isImeVisible = { true },
      )

    driver.onFinished()
    driver.onFinished()

    assertEquals(1, invalidations)
  }

  @Test
  fun notReadyControlAfterHiddenRequestUsesTheCancellationFallback() {
    val events = mutableListOf<String>()
    val control = RecordingAndroidImeAnimationControl().apply { isReady = false }
    val driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { events += "invalidated" },
        cancelControl = {},
        hideIme = { events += "hide" },
        isImeVisible = { true },
        endpointAnimator = ImmediateAndroidImeEndpointAnimator,
      )
    driver.finish(SoftwareKeyboardPresentationEndpoint.Hidden) { events += "accepted" }

    driver.onReady(control)

    assertEquals(listOf("hide", "accepted"), events)
  }

  @Test
  fun failureApplyingPendingProgressInvalidatesWithoutEscapingTheReadinessCallback() {
    var invalidations = 0
    val control =
      RecordingAndroidImeAnimationControl().apply {
        onSetInsetsAndAlpha = { error("control was cancelled") }
      }
    val driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { invalidations += 1 },
        cancelControl = {},
        hideIme = {},
        isImeVisible = { true },
      )
    driver.updateHiddenProgress(0.5f)

    driver.onReady(control)
    driver.updateHiddenProgress(0.75f)

    assertEquals(1, invalidations)
    assertEquals(1, control.frames.size)
  }

  @Test
  fun failureFinishingAfterReadinessUsesTheRequestedEndpointFallback() {
    val events = mutableListOf<String>()
    val control = RecordingAndroidImeAnimationControl().apply { onFinish = { error("cancelled") } }
    val driver =
      AndroidSoftwareKeyboardPresentationDriver(
        onInvalidated = { events += "invalidated" },
        cancelControl = {},
        hideIme = { events += "hide" },
        isImeVisible = { true },
        endpointAnimator = ImmediateAndroidImeEndpointAnimator,
      )
    driver.finish(SoftwareKeyboardPresentationEndpoint.Hidden) { events += "accepted" }

    driver.onReady(control)

    assertEquals(listOf("hide", "accepted"), events)
  }
}

private data class AndroidImeFrame(val insets: Insets, val alpha: Float, val fraction: Float)

private val ImmediateAndroidImeEndpointAnimator =
  AndroidImeEndpointAnimator { _, toHiddenProgress, onProgress, onFinished ->
    onProgress(toHiddenProgress)
    onFinished()
    AndroidImeEndpointAnimation {}
  }

private class ManualAndroidImeEndpointAnimator : AndroidImeEndpointAnimator {
  var fromHiddenProgress: Float? = null
    private set

  var toHiddenProgress: Float? = null
    private set

  private var onProgress: ((Float) -> Unit)? = null
  private var onFinished: (() -> Unit)? = null

  override fun animate(
    fromHiddenProgress: Float,
    toHiddenProgress: Float,
    onProgress: (Float) -> Unit,
    onFinished: () -> Unit,
  ): AndroidImeEndpointAnimation {
    this.fromHiddenProgress = fromHiddenProgress
    this.toHiddenProgress = toHiddenProgress
    this.onProgress = onProgress
    this.onFinished = onFinished
    return AndroidImeEndpointAnimation {
      this.onProgress = null
      this.onFinished = null
    }
  }

  fun updateProgress(hiddenProgress: Float) {
    checkNotNull(onProgress).invoke(hiddenProgress)
  }

  fun finish() {
    updateProgress(checkNotNull(toHiddenProgress))
    checkNotNull(onFinished).invoke()
  }
}

private class RecordingAndroidImeAnimationControl(
  override val hiddenStateInsets: Insets = Insets.NONE,
  override val shownStateInsets: Insets = Insets.NONE,
  override val currentAlpha: Float = 1f,
) : AndroidImeAnimationControl {
  override var isReady: Boolean = true
  var onSetInsetsAndAlpha: () -> Unit = {}
  var onFinish: () -> Unit = {}
  val frames = mutableListOf<AndroidImeFrame>()
  val finishes = mutableListOf<Boolean>()

  override fun setInsetsAndAlpha(insets: Insets, alpha: Float, fraction: Float) {
    frames += AndroidImeFrame(insets, alpha, fraction)
    onSetInsetsAndAlpha()
  }

  override fun finish(shown: Boolean) {
    finishes += shown
    onFinish()
  }
}
