package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class SoftwareKeyboardPresentationTest {
  @Test
  fun unavailableFactoryCannotAcquire() {
    val controller = SoftwareKeyboardPresentationController { null }

    assertNull(controller.acquire())
    assertEquals(SoftwareKeyboardInteractionState(), controller.interactionState)
  }

  @Test
  fun onlyOnePlatformInteractionOwnsTheDriver() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)

    val first = controller.acquire()
    val second = controller.acquire()

    assertTrue(first != null)
    assertTrue(second != null)
    assertEquals(1, factory.acquireCount)
    assertEquals(first.interactionId, controller.interactionState.activeInteractionId)
  }

  @Test
  fun multipleContributionsShareOneDriverAndPublishTheirMaximumProgress() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val first = requireNotNull(controller.acquire())
    val second = requireNotNull(controller.acquire())
    val driver = factory.drivers.single()

    first.updateHiddenProgress(0.6f)
    second.updateHiddenProgress(0.3f)

    assertEquals(1, factory.acquireCount)
    assertEquals(0.6f, controller.interactionState.hiddenProgress)
    assertEquals(0.6f, driver.progress.last())

    second.updateHiddenProgress(0.8f)

    assertEquals(0.8f, controller.interactionState.hiddenProgress)
    assertEquals(0.8f, driver.progress.last())
  }

  @Test
  fun shownRemovesOnlyItsContributionUntilTheLastContributionFinishes() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val first = requireNotNull(controller.acquire())
    val second = requireNotNull(controller.acquire())
    val driver = factory.drivers.single()
    first.updateHiddenProgress(0.7f)
    second.updateHiddenProgress(0.4f)

    first.finish(SoftwareKeyboardPresentationEndpoint.Shown)

    assertTrue(driver.endpoints.isEmpty())
    assertTrue(controller.interactionState.unresolved)
    assertEquals(0.4f, controller.interactionState.hiddenProgress)

    second.finish(SoftwareKeyboardPresentationEndpoint.Shown)

    assertEquals(listOf(SoftwareKeyboardPresentationEndpoint.Shown), driver.endpoints)
    assertTrue(controller.interactionState.unresolved)
    driver.acceptEndpoint()
    assertEquals(
      SoftwareKeyboardInteractionResolution.Shown,
      controller.interactionState.lastResolution,
    )
  }

  @Test
  fun hiddenFinishWinsGloballyAndMakesEveryContributionStale() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val first = requireNotNull(controller.acquire())
    val second = requireNotNull(controller.acquire())
    val driver = factory.drivers.single()

    second.finish(SoftwareKeyboardPresentationEndpoint.Hidden)
    first.updateHiddenProgress(0.9f)
    first.finish(SoftwareKeyboardPresentationEndpoint.Shown)
    first.dispose()

    assertEquals(listOf(SoftwareKeyboardPresentationEndpoint.Hidden), driver.endpoints)
    assertNull(controller.acquire())
    driver.acceptEndpoint()
    assertEquals(
      SoftwareKeyboardInteractionResolution.Hidden,
      controller.interactionState.lastResolution,
    )
  }

  @Test
  fun disposingAContributionFallsBackToTheRemainingProgressAndFinalDisposeAborts() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val first = requireNotNull(controller.acquire())
    val second = requireNotNull(controller.acquire())
    val driver = factory.drivers.single()
    first.updateHiddenProgress(0.8f)
    second.updateHiddenProgress(0.3f)

    first.dispose()

    assertTrue(controller.interactionState.unresolved)
    assertEquals(0.3f, controller.interactionState.hiddenProgress)
    assertEquals(0, driver.disposeCount)

    second.dispose()

    assertFalse(controller.interactionState.unresolved)
    assertEquals(1, driver.disposeCount)
    assertEquals(
      SoftwareKeyboardInteractionResolution.Aborted,
      controller.interactionState.lastResolution,
    )
  }

  @Test
  fun progressIsClampedWithoutReacquiringTheDriver() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val session = requireNotNull(controller.acquire())

    session.updateHiddenProgress(-0.2f)
    session.updateHiddenProgress(0.4f)
    session.updateHiddenProgress(0.4f)
    session.updateHiddenProgress(1.3f)

    assertEquals(listOf(0f, 0.4f, 0.4f, 1f), factory.drivers.single().progress)
    assertEquals(1f, controller.interactionState.hiddenProgress)
    assertEquals(1, factory.acquireCount)
  }

  @Test
  fun endpointRemainsUnresolvedUntilTheDriverAcceptsIt() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val session = requireNotNull(controller.acquire())
    val driver = factory.drivers.single()

    session.finish(SoftwareKeyboardPresentationEndpoint.Hidden)
    session.finish(SoftwareKeyboardPresentationEndpoint.Shown)
    session.updateHiddenProgress(0.5f)

    assertEquals(listOf(SoftwareKeyboardPresentationEndpoint.Hidden), driver.endpoints)
    assertTrue(controller.interactionState.unresolved)
    assertEquals(0L, controller.interactionState.resolutionVersion)
    assertNull(controller.interactionState.lastResolution)
    assertNull(controller.acquire())

    driver.acceptEndpoint()

    assertFalse(controller.interactionState.unresolved)
    assertEquals(1L, controller.interactionState.resolutionVersion)
    assertEquals(
      SoftwareKeyboardInteractionResolution.Hidden,
      controller.interactionState.lastResolution,
    )
    assertTrue(controller.acquire() != null)
  }

  @Test
  fun shownEndpointPublishesShownOnlyAfterAcceptance() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val session = requireNotNull(controller.acquire())
    val driver = factory.drivers.single()

    session.finish(SoftwareKeyboardPresentationEndpoint.Shown)
    assertNull(controller.interactionState.lastResolution)

    driver.acceptEndpoint()

    assertEquals(
      SoftwareKeyboardInteractionResolution.Shown,
      controller.interactionState.lastResolution,
    )
    assertEquals(1L, controller.interactionState.resolutionVersion)
  }

  @Test
  fun disposeAbortsOnceAndMakesTheStaleSessionHarmless() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val session = requireNotNull(controller.acquire())
    val driver = factory.drivers.single()

    session.dispose()
    session.dispose()
    session.updateHiddenProgress(0.7f)
    session.finish(SoftwareKeyboardPresentationEndpoint.Hidden)

    assertEquals(1, driver.disposeCount)
    assertTrue(driver.progress.isEmpty())
    assertTrue(driver.endpoints.isEmpty())
    assertEquals(
      SoftwareKeyboardInteractionResolution.Aborted,
      controller.interactionState.lastResolution,
    )
    assertEquals(1L, controller.interactionState.resolutionVersion)
  }

  @Test
  fun lateCallbacksFromAnOldGenerationDoNotAffectTheCurrentSession() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val firstSession = requireNotNull(controller.acquire())
    val firstDriver = factory.drivers.single()
    val firstInvalidation = factory.invalidations.single()

    firstSession.finish(SoftwareKeyboardPresentationEndpoint.Hidden)
    firstDriver.acceptEndpoint()
    val secondSession = requireNotNull(controller.acquire())
    val stateBeforeLateCallbacks = controller.interactionState

    firstDriver.acceptEndpoint()
    firstInvalidation()

    assertEquals(secondSession.interactionId, controller.interactionState.activeInteractionId)
    assertEquals(stateBeforeLateCallbacks, controller.interactionState)
  }

  @Test
  fun matchingDriverInvalidationAbortsTheSession() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    requireNotNull(controller.acquire())

    factory.invalidations.single()()

    assertFalse(controller.interactionState.unresolved)
    assertEquals(
      SoftwareKeyboardInteractionResolution.Aborted,
      controller.interactionState.lastResolution,
    )
    assertEquals(1L, controller.interactionState.resolutionVersion)
  }

  @Test
  fun acquiringIsUnresolvedBeforeTheFactoryRunsAndRejectsSynchronousReentry() {
    lateinit var controller: SoftwareKeyboardPresentationController
    var stateDuringFactory = SoftwareKeyboardInteractionState()
    var nestedSession: SoftwareKeyboardPresentationSession? = null
    val driver = RecordingDriver()
    controller = SoftwareKeyboardPresentationController { onInvalidated ->
      stateDuringFactory = controller.interactionState
      nestedSession = controller.acquire()
      onInvalidated()
      driver
    }

    assertNull(controller.acquire())

    assertTrue(stateDuringFactory.unresolved)
    assertNull(nestedSession)
    assertEquals(1, driver.disposeCount)
    assertEquals(
      SoftwareKeyboardInteractionResolution.Aborted,
      controller.interactionState.lastResolution,
    )
  }

  @Test
  fun disposingTheControllerDuringAcquisitionAbortsTheReturnedDriver() {
    lateinit var controller: SoftwareKeyboardPresentationController
    val driver = RecordingDriver()
    controller = SoftwareKeyboardPresentationController {
      controller.dispose()
      driver
    }

    assertNull(controller.acquire())

    assertEquals(1, driver.disposeCount)
    assertEquals(
      SoftwareKeyboardInteractionResolution.Aborted,
      controller.interactionState.lastResolution,
    )
  }

  @Test
  fun synchronousInvalidationDuringUpdateCannotOverwriteAReentrantSession() {
    lateinit var controller: SoftwareKeyboardPresentationController
    lateinit var replacement: SoftwareKeyboardPresentationSession
    var acquireCount = 0
    controller = SoftwareKeyboardPresentationController { onInvalidated ->
      acquireCount += 1
      if (acquireCount == 1) {
        InvalidatingUpdateDriver {
          onInvalidated()
          replacement = requireNotNull(controller.acquire())
        }
      } else {
        RecordingDriver()
      }
    }
    val original = requireNotNull(controller.acquire())

    original.updateHiddenProgress(0.8f)

    assertEquals(replacement.interactionId, controller.interactionState.activeInteractionId)
    assertEquals(0f, controller.interactionState.hiddenProgress)
  }

  @Test
  fun driverFailuresNeverEscapeAndAbortOnlyTheMatchingSession() {
    val driver = ThrowingDriver()
    val controller = SoftwareKeyboardPresentationController { driver }
    val session = requireNotNull(controller.acquire())

    session.updateHiddenProgress(0.5f)
    session.finish(SoftwareKeyboardPresentationEndpoint.Hidden)
    session.dispose()

    assertFalse(controller.interactionState.unresolved)
    assertEquals(
      SoftwareKeyboardInteractionResolution.Aborted,
      controller.interactionState.lastResolution,
    )
    assertEquals(1L, controller.interactionState.resolutionVersion)
  }

  @Test
  fun factoryFailuresNeverEscape() {
    val controller =
      SoftwareKeyboardPresentationController(
        SoftwareKeyboardPresentationDriverFactory { error("factory failure") }
      )

    assertNull(controller.acquire())
    assertEquals(
      SoftwareKeyboardInteractionResolution.Aborted,
      controller.interactionState.lastResolution,
    )
    assertEquals(1L, controller.interactionState.resolutionVersion)
  }

  @Test
  fun finishFailuresAbortTheMatchingInteraction() {
    val controller = SoftwareKeyboardPresentationController { FinishThrowingDriver() }
    val session = requireNotNull(controller.acquire())

    session.finish(SoftwareKeyboardPresentationEndpoint.Hidden)

    assertFalse(controller.interactionState.unresolved)
    assertEquals(
      SoftwareKeyboardInteractionResolution.Aborted,
      controller.interactionState.lastResolution,
    )
  }
}

private class RecordingDriverFactory : SoftwareKeyboardPresentationDriverFactory {
  val drivers = mutableListOf<RecordingDriver>()
  val invalidations = mutableListOf<() -> Unit>()
  var acquireCount = 0
    private set

  override fun acquire(onInvalidated: () -> Unit): SoftwareKeyboardPresentationDriver {
    acquireCount += 1
    invalidations += onInvalidated
    return RecordingDriver().also(drivers::add)
  }
}

private class RecordingDriver : SoftwareKeyboardPresentationDriver {
  val progress = mutableListOf<Float>()
  val endpoints = mutableListOf<SoftwareKeyboardPresentationEndpoint>()
  var disposeCount = 0
    private set

  private var endpointAcceptance: (() -> Unit)? = null

  override fun updateHiddenProgress(progress: Float) {
    this.progress += progress
  }

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    endpoints += endpoint
    endpointAcceptance = onAccepted
  }

  override fun dispose() {
    disposeCount += 1
  }

  fun acceptEndpoint() {
    endpointAcceptance?.invoke()
  }
}

private class ThrowingDriver : SoftwareKeyboardPresentationDriver {
  override fun updateHiddenProgress(progress: Float) {
    error("update failure")
  }

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    error("finish failure")
  }

  override fun dispose() {
    error("dispose failure")
  }
}

private class InvalidatingUpdateDriver(private val onUpdate: () -> Unit) :
  SoftwareKeyboardPresentationDriver {
  override fun updateHiddenProgress(progress: Float) {
    onUpdate()
  }

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) = Unit

  override fun dispose() = Unit
}

private class FinishThrowingDriver : SoftwareKeyboardPresentationDriver {
  override fun updateHiddenProgress(progress: Float) = Unit

  override fun finish(endpoint: SoftwareKeyboardPresentationEndpoint, onAccepted: () -> Unit) {
    error("finish failure")
  }

  override fun dispose() = Unit
}
