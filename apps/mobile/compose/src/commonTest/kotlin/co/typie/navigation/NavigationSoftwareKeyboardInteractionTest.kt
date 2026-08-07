package co.typie.navigation

import androidx.compose.runtime.snapshots.Snapshot
import co.typie.platform.SoftwareKeyboardInteractionResolution
import co.typie.platform.SoftwareKeyboardPresentationController
import co.typie.platform.SoftwareKeyboardPresentationDriver
import co.typie.platform.SoftwareKeyboardPresentationDriverFactory
import co.typie.platform.SoftwareKeyboardPresentationEndpoint
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.withTimeout

class NavigationSoftwareKeyboardInteractionTest {
  @Test
  fun forwardsProgressOnlyWhileAnInteractionIsActive() {
    val factory = RecordingDriverFactory()
    val interaction =
      NavigationSoftwareKeyboardInteraction(SoftwareKeyboardPresentationController(factory))

    interaction.updateHiddenProgress(0.4f)
    interaction.start()
    interaction.updateHiddenProgress(0.4f)
    interaction.updateHiddenProgress(0.7f)

    assertEquals(listOf(0.4f, 0.7f), requireNotNull(factory.driver).progress)
  }

  @Test
  fun separateVisualTransitionContinuesFromTheReleasedKeyboardProgress() {
    val factory = RecordingDriverFactory()
    val interaction =
      NavigationSoftwareKeyboardInteraction(SoftwareKeyboardPresentationController(factory))

    interaction.start()
    interaction.updateHiddenProgress(0.8f)
    interaction.continueFromHiddenProgress(0.8f)
    interaction.updateHiddenProgress(0f)
    interaction.updateHiddenProgress(0.5f)
    interaction.updateHiddenProgress(1f)

    val firstDriver = requireNotNull(factory.driver)
    assertEquals(4, firstDriver.progress.size)
    listOf(0.8f, 0.8f, 0.9f, 1f).zip(firstDriver.progress).forEach { (expected, actual) ->
      assertEquals(expected, actual, absoluteTolerance = 0.0001f)
    }

    interaction.restore()
    firstDriver.acceptEndpoint()
    interaction.start()
    interaction.updateHiddenProgress(0.25f)

    assertEquals(listOf(0.25f), requireNotNull(factory.driver).progress)
  }

  @Test
  fun restoreRemovesOnlyNavigationContributionAndRevealsTheRemainingProgress() {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val editorContribution = requireNotNull(controller.acquire())
    editorContribution.updateHiddenProgress(0.4f)
    val interaction = NavigationSoftwareKeyboardInteraction(controller)
    interaction.start()
    interaction.updateHiddenProgress(0.8f)

    assertEquals(0.8f, controller.interactionState.hiddenProgress)

    interaction.restore()

    assertEquals(0.4f, controller.interactionState.hiddenProgress)
    assertEquals(emptyList(), requireNotNull(factory.driver).endpoints)
  }

  @Test
  fun hideWaitsForTheGlobalHiddenResolution() = runTest {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val interaction = NavigationSoftwareKeyboardInteraction(controller)
    interaction.start()

    val resolution =
      async(start = CoroutineStart.UNDISPATCHED) { interaction.hideAndAwaitResolution() }

    assertFalse(resolution.isCompleted)
    requireNotNull(factory.driver).acceptEndpoint()
    Snapshot.sendApplyNotifications()
    assertEquals(SoftwareKeyboardInteractionResolution.Hidden, resolution.await())
  }

  @Test
  fun hideWaitReleasesWhenThePlatformInteractionAborts() = runTest {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val interaction = NavigationSoftwareKeyboardInteraction(controller)
    interaction.start()

    val resolution =
      async(start = CoroutineStart.UNDISPATCHED) { interaction.hideAndAwaitResolution() }

    assertFalse(resolution.isCompleted)
    requireNotNull(factory.invalidation).invoke()
    Snapshot.sendApplyNotifications()
    assertEquals(SoftwareKeyboardInteractionResolution.Aborted, resolution.await())
  }

  @Test
  fun staleNavigationContributionDoesNotWaitForAnUnrelatedResolution() = runTest {
    val factory = RecordingDriverFactory()
    val controller = SoftwareKeyboardPresentationController(factory)
    val interaction = NavigationSoftwareKeyboardInteraction(controller)
    interaction.start()
    requireNotNull(factory.invalidation).invoke()
    val unrelated = requireNotNull(controller.acquire())
    unrelated.finish(SoftwareKeyboardPresentationEndpoint.Hidden)
    requireNotNull(factory.driver).acceptEndpoint()

    assertEquals(null, withTimeout(100) { interaction.hideAndAwaitResolution() })
  }

  @Test
  fun restoreResolvesToShown() {
    val restoreFactory = RecordingDriverFactory()
    val restoreInteraction =
      NavigationSoftwareKeyboardInteraction(SoftwareKeyboardPresentationController(restoreFactory))
    restoreInteraction.start()

    restoreInteraction.restore()

    assertEquals(
      listOf(SoftwareKeyboardPresentationEndpoint.Shown),
      requireNotNull(restoreFactory.driver).endpoints,
    )
  }

  @Test
  fun unresolvedContributionIsAbortedWhenTheOwnerIsDisposed() {
    val factory = RecordingDriverFactory()
    val interaction =
      NavigationSoftwareKeyboardInteraction(SoftwareKeyboardPresentationController(factory))
    interaction.start()

    interaction.dispose()

    assertEquals(1, requireNotNull(factory.driver).disposeCount)
  }

  @Test
  fun unavailableControllerKeepsEveryOperationANoOp() = runTest {
    val controller = SoftwareKeyboardPresentationController.unavailable
    val interaction = NavigationSoftwareKeyboardInteraction(controller)

    interaction.start()
    interaction.updateHiddenProgress(0.5f)
    interaction.restore()
    interaction.hideAndAwaitResolution()
    interaction.dispose()

    assertEquals(null, controller.interactionState.activeInteractionId)
  }
}

private class RecordingDriverFactory : SoftwareKeyboardPresentationDriverFactory {
  var driver: RecordingDriver? = null
    private set

  var invalidation: (() -> Unit)? = null
    private set

  override fun acquire(onInvalidated: () -> Unit): SoftwareKeyboardPresentationDriver =
    RecordingDriver().also {
      driver = it
      invalidation = onInvalidated
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
