package co.typie.navigation

import co.typie.route.Route
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.test.runTest

class RouteRemovalCoordinatorTest {
  @Test
  fun delayedPreparationRemainsOwnedWhileRouteBecomesVisible() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val editorRoute = Route.Editor("editor")
    var delayedRoute: Route? = null
    var currentDuringDelayed = false
    val editor =
      RecordingRemovalInterceptor(
        preparation = RouteRemovalPreparation.Ready,
        onPrepare = { onDelayed -> checkNotNull(onDelayed).invoke() },
      )
    coordinator.register(editorRoute, editor)

    val segment =
      coordinator.prepareSegment(
        routesToRemove = listOf(editorRoute),
        requestedTarget = Route.Home,
        onDelayed = { route ->
          delayedRoute = route
          currentDuringDelayed = coordinator.activeSegmentIsCurrent()
        },
      )

    assertEquals(editorRoute, delayedRoute)
    assertTrue(currentDuringDelayed)
    assertEquals(Route.Home, segment.destination)
  }

  @Test
  fun preparationWithoutDelayedCallbackDoesNotProvidePresentationHook() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val editorRoute = Route.Editor("editor")
    var receivedCallback = true
    coordinator.register(
      editorRoute,
      RecordingRemovalInterceptor(
        preparation = RouteRemovalPreparation.Ready,
        onPrepare = { onDelayed -> receivedCallback = onDelayed != null },
      ),
    )

    coordinator.prepareSegment(listOf(editorRoute), Route.Home)

    assertEquals(false, receivedCallback)
  }

  @Test
  fun hiddenFailureStopsAtEditorAfterCommittingReadyPrefix() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val top = RecordingRemovalInterceptor(RouteRemovalPreparation.Ready)
    val editor = RecordingRemovalInterceptor(RouteRemovalPreparation.NeedsDecision)
    val topRoute = Route.Document("top")
    val editorRoute = Route.Editor("editor")
    val target = Route.Home
    coordinator.register(topRoute, top)
    coordinator.register(editorRoute, editor)

    val segment =
      coordinator.prepareSegment(
        routesToRemove = listOf(topRoute, editorRoute),
        requestedTarget = target,
      )

    assertEquals(editorRoute, segment.destination)
    assertEquals(editorRoute, segment.blockedRoute)
    coordinator.commitReadyPrefix()
    editor.decision = RouteRemovalDecision.CancelRemoval
    assertEquals(NavigationResult.StoppedAt(editorRoute), coordinator.resolveBlockedRoute())
    assertEquals(0, top.rollbacks)
    assertEquals(1, editor.rollbacks)
  }

  @Test
  fun approvedRemovalContinuesTowardOriginalTarget() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val editorRoute = Route.Editor("editor")
    val target = Route.Home
    val editor = RecordingRemovalInterceptor(RouteRemovalPreparation.NeedsDecision)
    coordinator.register(editorRoute, editor)

    coordinator.prepareSegment(listOf(editorRoute), target)
    coordinator.commitReadyPrefix()
    editor.decision = RouteRemovalDecision.ProceedWithRemoval
    assertEquals(null, coordinator.resolveBlockedRoute())
    assertEquals(0, editor.rollbacks)

    val continued = coordinator.prepareSegment(listOf(editorRoute), target)
    assertEquals(target, continued.destination)
    assertEquals(null, continued.blockedRoute)
  }

  @Test
  fun replacementAfterRemovalApprovalRequiresFreshPreparation() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val editorRoute = Route.Editor("editor")
    val original = RecordingRemovalInterceptor(RouteRemovalPreparation.NeedsDecision)
    coordinator.register(editorRoute, original)
    coordinator.prepareSegment(listOf(editorRoute), Route.Home)
    coordinator.commitReadyPrefix()
    original.decision = RouteRemovalDecision.ProceedWithRemoval
    coordinator.resolveBlockedRoute()

    val replacement = RecordingRemovalInterceptor(RouteRemovalPreparation.Ready)
    coordinator.register(editorRoute, replacement)
    val continued = coordinator.prepareSegment(listOf(editorRoute), Route.Home)

    assertEquals(1, original.rollbacks)
    assertEquals(1, replacement.prepares)
    assertEquals(Route.Home, continued.destination)
  }

  @Test
  fun rollbackAfterRemovalApprovalResumesApprovedInterceptor() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val editorRoute = Route.Editor("editor")
    val editor = RecordingRemovalInterceptor(RouteRemovalPreparation.NeedsDecision)
    coordinator.register(editorRoute, editor)
    coordinator.prepareSegment(listOf(editorRoute), Route.Home)
    coordinator.commitReadyPrefix()
    editor.decision = RouteRemovalDecision.ProceedWithRemoval
    coordinator.resolveBlockedRoute()

    coordinator.rollbackActiveSegment()

    assertEquals(1, editor.rollbacks)
  }

  @Test
  fun replacedReadyInterceptorInvalidatesActiveSegment() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val editorRoute = Route.Editor("editor")
    val original = RecordingRemovalInterceptor(RouteRemovalPreparation.Ready)
    coordinator.register(editorRoute, original)
    coordinator.prepareSegment(listOf(editorRoute), Route.Home)

    coordinator.register(editorRoute, RecordingRemovalInterceptor(RouteRemovalPreparation.Ready))

    assertEquals(false, coordinator.activeSegmentIsCurrent())
    coordinator.rollbackActiveSegment()
    assertEquals(1, original.rollbacks)
  }

  @Test
  fun replacedBlockedInterceptorRestartsPreparationWithoutPromptingStaleHandler() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val editorRoute = Route.Editor("editor")
    val original = RecordingRemovalInterceptor(RouteRemovalPreparation.NeedsDecision)
    coordinator.register(editorRoute, original)
    coordinator.prepareSegment(listOf(editorRoute), Route.Home)
    coordinator.commitReadyPrefix()
    coordinator.register(editorRoute, RecordingRemovalInterceptor(RouteRemovalPreparation.Ready))

    assertEquals(null, coordinator.resolveBlockedRoute())
    assertEquals(1, original.rollbacks)
  }

  @Test
  fun unregisteredBlockedInterceptorRestartsWithoutPromptingStaleHandler() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val editorRoute = Route.Editor("editor")
    val original = RecordingRemovalInterceptor(RouteRemovalPreparation.NeedsDecision)
    val unregister = coordinator.register(editorRoute, original)
    coordinator.prepareSegment(listOf(editorRoute), Route.Home)
    coordinator.commitReadyPrefix()

    unregister()

    assertEquals(null, coordinator.resolveBlockedRoute())
    assertEquals(1, original.rollbacks)
  }

  @Test
  fun rollbackAttemptsEveryPreparedRouteAndPreservesFirstFailure() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val firstFailure = IllegalStateException("first rollback failure")
    val secondFailure = IllegalStateException("second rollback failure")
    val secondToRollback =
      RecordingRemovalInterceptor(
        preparation = RouteRemovalPreparation.Ready,
        rollbackFailure = secondFailure,
      )
    val firstToRollback =
      RecordingRemovalInterceptor(
        preparation = RouteRemovalPreparation.Ready,
        rollbackFailure = firstFailure,
      )
    val secondRoute = Route.Document("second")
    val firstRoute = Route.Editor("first")
    coordinator.register(secondRoute, secondToRollback)
    coordinator.register(firstRoute, firstToRollback)
    coordinator.prepareSegment(listOf(secondRoute, firstRoute), Route.Home)

    val thrown = assertFailsWith<IllegalStateException> { coordinator.rollbackActiveSegment() }

    assertSame(firstFailure, thrown)
    assertSame(secondFailure, thrown.suppressed.single())
    assertEquals(1, firstToRollback.rollbacks)
    assertEquals(1, secondToRollback.rollbacks)
  }

  @Test
  fun preparationFailureRemainsPrimaryWhenRollbackAlsoFails() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val preparationFailure = IllegalStateException("preparation failure")
    val rollbackFailure = IllegalStateException("rollback failure")
    val route = Route.Editor("editor")
    coordinator.register(
      route,
      RecordingRemovalInterceptor(
        preparation = RouteRemovalPreparation.Ready,
        preparationFailure = preparationFailure,
        rollbackFailure = rollbackFailure,
      ),
    )

    val thrown =
      assertFailsWith<IllegalStateException> {
        coordinator.prepareSegment(listOf(route), Route.Home)
      }

    assertSame(preparationFailure, thrown)
    assertSame(rollbackFailure, thrown.suppressed.single())
  }

  @Test
  fun preparationCancellationRollsBackEveryPreparedRouteBeforeRethrowing() = runTest {
    val coordinator = RouteRemovalCoordinator()
    val cancellation = CancellationException("preparation cancelled")
    val first = RecordingRemovalInterceptor(RouteRemovalPreparation.Ready)
    val second =
      RecordingRemovalInterceptor(
        preparation = RouteRemovalPreparation.Ready,
        preparationFailure = cancellation,
      )
    val firstRoute = Route.Document("first")
    val secondRoute = Route.Editor("second")
    coordinator.register(firstRoute, first)
    coordinator.register(secondRoute, second)

    val thrown =
      assertFailsWith<CancellationException> {
        coordinator.prepareSegment(listOf(firstRoute, secondRoute), Route.Home)
      }

    assertSame(cancellation, thrown)
    assertEquals(1, first.rollbacks)
    assertEquals(1, second.rollbacks)
  }
}

private class RecordingRemovalInterceptor(
  private val preparation: RouteRemovalPreparation,
  private val preparationFailure: Throwable? = null,
  private val rollbackFailure: Throwable? = null,
  private val onPrepare: suspend ((suspend () -> Unit)?) -> Unit = {},
) : RouteRemovalInterceptor {
  var decision: RouteRemovalDecision = RouteRemovalDecision.CancelRemoval
  var prepares: Int = 0
  var rollbacks: Int = 0

  override suspend fun prepare(onDelayed: (suspend () -> Unit)?): RouteRemovalPreparation {
    prepares++
    onPrepare(onDelayed)
    preparationFailure?.let { throw it }
    return preparation
  }

  override suspend fun resolveDecision(): RouteRemovalDecision = decision

  override suspend fun rollback() {
    rollbacks++
    rollbackFailure?.let { throw it }
  }
}
