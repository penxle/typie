package co.typie.navigation

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.Spring.StiffnessMediumLow
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.ProvidedValue
import androidx.compose.runtime.getValue
import androidx.compose.runtime.movableContentOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerInputChange
import androidx.compose.ui.input.pointer.PointerInputScope
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChangeIgnoreConsumed
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.input.pointer.util.addPointerInputChange
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalViewConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import androidx.compose.ui.util.fastFirstOrNull
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.touchlab.kermit.Logger
import co.typie.ext.pointerIgnore
import co.typie.platform.isTouchDragPointer
import co.typie.route.Route
import co.typie.route.RouteTransitionStyle
import co.typie.route.keepAlive
import co.typie.route.popGestureDisabled
import co.typie.route.transitionStyleTo
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.bottombar.LocalBottomBarAnimationSource
import co.typie.ui.component.bottombar.LocalBottomBarState
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.topbar.LocalTopBarAnimationSource
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.NavDirection
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import dev.chrisbanes.haze.HazeState
import dev.chrisbanes.haze.hazeSource
import kotlin.math.abs
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private enum class AnimState {
  Idle,
  Push,
  Pop,
  Dragging,
  PopGestureCommitted,
}

private class NavigationRouteScene(
  val owner: ViewModelStoreOwner,
  val foregroundRegistry: NavigationForegroundRegistry,
  val content: @Composable (TopBarState?, BottomBarState?) -> Unit,
)

private const val NavigationPopActivationSlopMultiplier = 3f

internal fun shouldCommitNavigationPop(
  progress: Float,
  velocityX: Float,
  containerWidth: Float,
): Boolean {
  val logicalVelocity = velocityX / containerWidth
  return if (abs(logicalVelocity) >= 1f) logicalVelocity > 0f else progress > 0.5f
}

private class NavigationPopPointerVelocity {
  private val tracker = VelocityTracker()

  fun update(pressedPointerCount: Int, change: PointerInputChange?) {
    if (pressedPointerCount > 1) {
      reset()
      return
    }
    change?.let { tracker.addPointerInputChange(it) }
  }

  fun release(maximumVelocity: Float): Float {
    val velocityX = tracker.calculateVelocity(Velocity(maximumVelocity, maximumVelocity)).x
    reset()
    return velocityX
  }

  fun reset() {
    tracker.resetTracking()
  }
}

private suspend fun PointerInputScope.detectNavigationPopDrag(
  activationDistance: Float,
  pointerVelocity: NavigationPopPointerVelocity,
  canStart: () -> Boolean,
  isSequenceRejected: () -> Boolean,
  onStart: () -> Unit,
  onDrag: (Float) -> Unit,
  onRelease: (Float) -> Unit,
  onCancel: () -> Unit,
) {
  awaitEachGesture {
    val down = awaitFirstDown(requireUnconsumed = false)
    if (!down.type.isTouchDragPointer() || !canStart()) return@awaitEachGesture

    var activationOvershootX = 0f
    while (true) {
      val event = awaitPointerEvent(PointerEventPass.Main)
      if (event.changes.count { it.pressed } != 1) return@awaitEachGesture
      val change = event.changes.fastFirstOrNull { it.id == down.id } ?: return@awaitEachGesture
      if (!change.pressed) return@awaitEachGesture
      if (change.isConsumed) return@awaitEachGesture

      when (
        val activation =
          resolveNavigationPopActivation(
            dragFromStart = change.position - down.position,
            activationDistance = activationDistance,
          )
      ) {
        NavigationPopActivation.Pending -> continue
        NavigationPopActivation.Rejected -> return@awaitEachGesture
        is NavigationPopActivation.Ready -> {
          if (isSequenceRejected() || !canStart() || change.isConsumed) {
            return@awaitEachGesture
          }
          change.consume()
          activationOvershootX = activation.overshootX
          break
        }
      }
    }

    onStart()
    onDrag(activationOvershootX)
    while (true) {
      if (isSequenceRejected()) {
        pointerVelocity.reset()
        onCancel()
        return@awaitEachGesture
      }

      val event = awaitPointerEvent(PointerEventPass.Main)
      if (event.changes.count { it.pressed } > 1) {
        pointerVelocity.reset()
        onCancel()
        return@awaitEachGesture
      }
      val change = event.changes.fastFirstOrNull { it.id == down.id }
      if (change == null) {
        pointerVelocity.reset()
        onRelease(0f)
        return@awaitEachGesture
      }

      onDrag(change.positionChangeIgnoreConsumed().x)
      if (!change.pressed) {
        onRelease(pointerVelocity.release(viewConfiguration.maximumFlingVelocity))
        return@awaitEachGesture
      }
      change.consume()
    }
  }
}

@Composable
fun NavigationStack(
  navigator: Navigator,
  topBarState: TopBarState,
  bottomBarState: BottomBarState? = null,
  modifier: Modifier = Modifier,
  content: @Composable (Route) -> Unit,
) {
  val exitTopBarState = remember { TopBarState() }
  val exitBottomBarState = remember { bottomBarState?.let { BottomBarState() } }

  // 라우트별 movable content 캐시. 메인/비하인드 슬롯 사이를 오가도
  // 동일한 composition 인스턴스를 유지해 currentCompositeKeyHashCode가
  // 안정적으로 유지된다. (같은 스크린을 두 call site에서 composition하면
  // compound key가 달라져 viewModel/rememberSaveable 키가 꼬인다.)
  val latestContent by rememberUpdatedState(content)
  val routeScenes = remember { mutableMapOf<Route, NavigationRouteScene>() }
  val routeSceneFor: (Route) -> NavigationRouteScene = { route ->
    routeScenes.getOrPut(route) {
      val owner =
        object : ViewModelStoreOwner {
          override val viewModelStore = navigator.viewModelStoreFor(route)
        }
      val foregroundRegistry = NavigationForegroundRegistry()
      val routeContent =
        movableContentOf<TopBarState?, BottomBarState?> { topBar, bottomBar ->
          val providers =
            buildList<ProvidedValue<*>> {
              add(LocalViewModelStoreOwner provides owner)
              add(LocalRoute provides route)
              add(LocalTopBarState provides topBar)
              if (bottomBar != null) add(LocalBottomBarState provides bottomBar)
            }
          CompositionLocalProvider(*providers.toTypedArray()) {
            ProvideBottomBar(enabled = false)
            ProvideNavigationForegroundRegistry(foregroundRegistry) { latestContent(route) }
          }
        }
      NavigationRouteScene(
        owner = owner,
        foregroundRegistry = foregroundRegistry,
        content = routeContent,
      )
    }
  }
  val presentRouteSurface: @Composable (Route, TopBarState?, BottomBarState?) -> Unit =
    { route, topBar, bottomBar ->
      val scene = routeSceneFor(route)
      scene.content(topBar, bottomBar)
    }
  val presentRouteForeground: @Composable (Route, TopBarState?, BottomBarState?, Modifier) -> Unit =
    { route, topBar, bottomBar, foregroundModifier ->
      val scene = routeSceneFor(route)
      scene.foregroundRegistry.Content(
        route = route,
        viewModelStoreOwner = scene.owner,
        topBarState = topBar,
        bottomBarState = bottomBar,
        modifier = foregroundModifier,
      )
    }

  // 백스택에서 제거된 라우트의 movable 캐시 정리
  LaunchedEffect(Unit) {
    snapshotFlow { navigator.stack.toSet() }
      .collect { active -> routeScenes.keys.retainAll(active) }
  }

  val scope = rememberCoroutineScope()
  var containerWidth by remember { mutableStateOf(0f) }
  var containerHeight by remember { mutableStateOf(0f) }
  var animState by remember { mutableStateOf(AnimState.Idle) }

  // visibleRoute: Idle 상태에서 보이는 화면. 애니메이션 완료 후 업데이트.
  var visibleRoute by remember { mutableStateOf(navigator.current) }
  // behindRoute: 애니메이션/제스처 중 뒤에 깔리는 화면. Idle에서는 null.
  var behindRoute by remember { mutableStateOf<Route?>(null) }
  var transitionStyle by remember { mutableStateOf(RouteTransitionStyle.Slide) }

  val progress = remember { Animatable(0f) }
  val topBarBackdropHazeState = remember { HazeState() }

  val popNestedScroll = remember { NavigationPopNestedScroll() }
  val navigationPopActivationDistance =
    LocalViewConfiguration.current.touchSlop * NavigationPopActivationSlopMultiplier
  val backGestureZoneWidth by rememberUpdatedState(systemBackGestureZoneWidth())
  val popPointerVelocity = remember { NavigationPopPointerVelocity() }
  var predictiveBackActive by remember { mutableStateOf(false) }

  fun canStartPopGesture(): Boolean =
    navigator.canPop &&
      !navigator.current.popGestureDisabled &&
      !navigator.isTransitioning &&
      !predictiveBackActive &&
      containerWidth > 0f &&
      (animState == AnimState.Idle || animState == AnimState.Dragging)

  fun clearRemovedRoutes(removedRoutes: List<Route>) {
    removedRoutes.forEach { removedRoute ->
      topBarState.clearRoute(removedRoute)
      exitTopBarState.clearRoute(removedRoute)
      bottomBarState?.clearRoute(removedRoute)
      exitBottomBarState?.clearRoute(removedRoute)
    }
  }

  suspend fun settleAtCurrentRoute() {
    progress.snapTo(0f)
    visibleRoute = navigator.current
    behindRoute = null
    animState = AnimState.Idle
  }

  fun commitRemovalTo(target: Route) {
    val removedRoutes = navigator.performPopTo(target)
    visibleRoute = navigator.current
    behindRoute = null
    animState = AnimState.Idle
    clearRemovedRoutes(removedRoutes)
  }

  suspend fun animateRemovalTo(target: Route, verifyPreparedSegment: Boolean = true): Boolean {
    val requiresTransition = target != navigator.current
    val continuesPopGesture =
      animState == AnimState.PopGestureCommitted &&
        behindRoute == target &&
        transitionStyle != RouteTransitionStyle.Fade
    if (!requiresTransition) {
      if (animState == AnimState.PopGestureCommitted) {
        progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
      }
      settleAtCurrentRoute()
    } else {
      transitionStyle = visibleRoute.transitionStyleTo(target)
      behindRoute = target
      animState = AnimState.Pop
      if (continuesPopGesture) {
        progress.animateTo(1f, spring(stiffness = StiffnessMediumLow))
      } else {
        progress.snapTo(0f)
        progress.animateTo(1f, tween(350, easing = FastOutSlowInEasing))
      }
    }

    if (verifyPreparedSegment && !navigator.routeRemovals.activeSegmentIsCurrent()) {
      navigator.routeRemovals.rollbackActiveSegment()
      settleAtCurrentRoute()
      return false
    }

    if (!requiresTransition) return true
    commitRemovalTo(target)
    return true
  }

  suspend fun performProgressiveRemoval(target: Route): NavigationResult {
    while (navigator.current != target) {
      val targetIndex = navigator.stack.lastIndexOf(target)
      check(targetIndex >= 0) { "Removal target is not in the navigation stack" }
      val routesToRemove =
        navigator.stack.subList(targetIndex + 1, navigator.stack.size).asReversed()
      val segment =
        navigator.routeRemovals.prepareSegment(routesToRemove, target) { delayedRoute ->
          animateRemovalTo(delayedRoute, verifyPreparedSegment = false)
          navigator.routeRemovals.commitReadyPrefix()
        }
      if (!animateRemovalTo(segment.destination)) continue

      if (segment.blockedRoute == null) {
        navigator.routeRemovals.commitSegment()
        return NavigationResult.ReachedTarget
      }

      navigator.routeRemovals.commitReadyPrefix()
      navigator.routeRemovals.resolveBlockedRoute()?.let {
        return it
      }
    }
    navigator.routeRemovals.commitSegment()
    return NavigationResult.ReachedTarget
  }

  suspend fun performCommittedGestureRemoval(target: Route): NavigationResult = coroutineScope {
    var delayed = false
    val exitAnimation =
      async(start = CoroutineStart.UNDISPATCHED) {
        progress.animateTo(1f, spring(stiffness = StiffnessMediumLow))
      }

    suspend fun settleGestureAtCurrentRoute() {
      exitAnimation.cancelAndJoin()
      progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
      settleAtCurrentRoute()
    }

    suspend fun rollbackGestureAndRetry(): NavigationResult {
      try {
        navigator.routeRemovals.rollbackActiveSegment()
      } finally {
        settleGestureAtCurrentRoute()
      }
      return performProgressiveRemoval(target)
    }

    try {
      val targetIndex = navigator.stack.lastIndexOf(target)
      check(targetIndex >= 0) { "Removal target is not in the navigation stack" }
      val routesToRemove =
        navigator.stack.subList(targetIndex + 1, navigator.stack.size).asReversed()
      val segment =
        navigator.routeRemovals.prepareSegment(routesToRemove, target) {
          settleGestureAtCurrentRoute()
          delayed = true
        }

      if (!navigator.routeRemovals.activeSegmentIsCurrent()) {
        return@coroutineScope rollbackGestureAndRetry()
      }

      if (segment.blockedRoute == null) {
        if (delayed) {
          if (!animateRemovalTo(target)) {
            return@coroutineScope performProgressiveRemoval(target)
          }
        } else {
          exitAnimation.await()
        }
        if (!navigator.routeRemovals.activeSegmentIsCurrent()) {
          return@coroutineScope rollbackGestureAndRetry()
        }

        if (!delayed) commitRemovalTo(target)
        navigator.routeRemovals.commitSegment()
        return@coroutineScope NavigationResult.ReachedTarget
      }

      if (!delayed) settleGestureAtCurrentRoute()
      if (!navigator.routeRemovals.activeSegmentIsCurrent()) {
        return@coroutineScope rollbackGestureAndRetry()
      }

      navigator.routeRemovals.commitReadyPrefix()
      navigator.routeRemovals.resolveBlockedRoute()?.let {
        return@coroutineScope it
      }
      performProgressiveRemoval(target)
    } catch (throwable: Throwable) {
      withContext(NonCancellable) { settleGestureAtCurrentRoute() }
      throw throwable
    }
  }

  fun startPopDrag() {
    val prev = navigator.previous ?: return
    transitionStyle = visibleRoute.transitionStyleTo(prev)
    behindRoute = prev
    animState = AnimState.Dragging
    scope.launch { progress.snapTo(0f) }
  }

  suspend fun startPredictiveBackDrag(): Boolean {
    if (navigator.isTransitioning || animState != AnimState.Idle) return false
    val prev = navigator.previous ?: return false
    transitionStyle = visibleRoute.transitionStyleTo(prev)
    behindRoute = prev
    animState = AnimState.Dragging
    predictiveBackActive = true
    progress.snapTo(0f)
    return true
  }

  fun updatePopDrag(dragAmount: Float) {
    scope.launch {
      val newValue = (progress.value + dragAmount / containerWidth).coerceIn(0f, 1f)
      progress.snapTo(newValue)
    }
  }

  suspend fun commitPopDrag() {
    val target = navigator.previous
    if (target != null) {
      try {
        animState = AnimState.PopGestureCommitted
        if (navigator.pop() == NavigationResult.NotStarted) {
          progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
          settleAtCurrentRoute()
        }
      } catch (e: CancellationException) {
        withContext(NonCancellable) { settleAtCurrentRoute() }
        throw e
      } catch (e: Throwable) {
        settleAtCurrentRoute()
        Logger.e(e) { "Navigation gesture removal failed" }
      }
    } else {
      progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
      settleAtCurrentRoute()
    }
  }

  fun finishPopDrag(velocityX: Float) {
    scope.launch {
      if (
        shouldCommitNavigationPop(
          progress = progress.value,
          velocityX = velocityX,
          containerWidth = containerWidth,
        )
      ) {
        commitPopDrag()
      } else {
        progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
        behindRoute = null
        animState = AnimState.Idle
      }
    }
  }

  fun cancelPopDrag() {
    if (animState != AnimState.Dragging) return
    scope.launch {
      progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
      behindRoute = null
      animState = AnimState.Idle
    }
  }

  popNestedScroll.update(
    activationDistance = navigationPopActivationDistance,
    canStart = ::canStartPopGesture,
    onStart = ::startPopDrag,
    onDrag = ::updatePopDrag,
    onRelease = ::finishPopDrag,
    onCancel = ::cancelPopDrag,
  )

  // pop 요청은 이 collector가 애니메이션과 stack 변경을 함께 처리한다.
  LaunchedEffect(Unit) {
    snapshotFlow { navigator.peekPopTarget() to animState }
      .collect { (targetRoute, currentAnimState) ->
        if (
          targetRoute != null &&
            (currentAnimState == AnimState.Idle ||
              (currentAnimState == AnimState.PopGestureCommitted && targetRoute == behindRoute))
        ) {
          try {
            val result =
              when (navigator.peekRemovalPolicy()) {
                RouteRemovalPolicy.Intercept ->
                  if (
                    currentAnimState == AnimState.PopGestureCommitted && targetRoute == behindRoute
                  ) {
                    performCommittedGestureRemoval(targetRoute)
                  } else {
                    performProgressiveRemoval(targetRoute)
                  }
                RouteRemovalPolicy.BypassInterceptors -> {
                  check(animateRemovalTo(targetRoute))
                  NavigationResult.ReachedTarget
                }
              }
            navigator.consumePopRequest()
            navigator.completeTransition(result = result)
          } catch (e: Throwable) {
            withContext(NonCancellable) {
              if (navigator.peekRemovalPolicy() == RouteRemovalPolicy.BypassInterceptors) {
                // Server deletion already succeeded. Do not strand the deleted document because
                // presentation failed; finish the exact prepared removal without another prompt.
                val removedRoutes = navigator.performPopTo(targetRoute)
                settleAtCurrentRoute()
                clearRemovedRoutes(removedRoutes)
                navigator.consumePopRequest()
                navigator.completeTransition()
              } else {
                val rollbackFailure =
                  try {
                    navigator.routeRemovals.rollbackActiveSegment()
                    null
                  } catch (throwable: Throwable) {
                    throwable
                  }
                val settleFailure =
                  try {
                    settleAtCurrentRoute()
                    null
                  } catch (throwable: Throwable) {
                    throwable
                  }
                listOfNotNull(rollbackFailure, settleFailure).forEach { cleanupFailure ->
                  if (cleanupFailure !== e) e.addSuppressed(cleanupFailure)
                }
                navigator.consumePopRequest()
                navigator.completeTransition(e)
              }
            }
            if (e is CancellationException) throw e
          }
        }
      }
  }

  // Push 및 직접 pop() 호출 처리
  LaunchedEffect(navigator.current) {
    if (navigator.current != visibleRoute) {
      when (navigator.lastOperation) {
        NavOperation.Push -> {
          // Push: visibleRoute(이전 화면)가 뒤로, navigator.current(새 화면)가 앞으로
          transitionStyle = visibleRoute.transitionStyleTo(navigator.current)
          behindRoute = visibleRoute
          animState = AnimState.Push
          progress.snapTo(0f)
          withFrameNanos {} // 새 화면의 첫 composition 완료 대기
          progress.animateTo(1f, tween(350, easing = FastOutSlowInEasing))
          visibleRoute = navigator.current
          navigator.completeTransition()
        }

        else -> {
          // Pop: visibleRoute(현재 화면)가 앞에서 나가고, navigator.current(이전 화면)가 뒤에서 나타남
          val poppedRoute = visibleRoute
          transitionStyle = poppedRoute.transitionStyleTo(navigator.current)
          behindRoute = navigator.current
          animState = AnimState.Pop
          progress.snapTo(0f)
          progress.animateTo(1f, tween(350, easing = FastOutSlowInEasing))
          visibleRoute = navigator.current
          clearRemovedRoutes(listOf(poppedRoute))
          navigator.completeTransition()
        }
      }
      behindRoute = null
      animState = AnimState.Idle
    }
  }

  val animationProviders =
    buildList<ProvidedValue<*>> {
      add(Nav provides navigator)
      add(LocalNavigationPopNestedScroll provides popNestedScroll)
      add(LocalTopBarAnimationSource provides topBarState)
      bottomBarState?.let { add(LocalBottomBarAnimationSource provides it) }
    }
  CompositionLocalProvider(*animationProviders.toTypedArray()) {
    PlatformPredictiveBackHandler(enabled = navigator.canPop) { events ->
      var interactive = false
      try {
        events.collect { value ->
          if (!interactive) {
            interactive = startPredictiveBackDrag()
          }
          if (interactive) {
            progress.snapTo(value)
          }
        }
      } catch (e: CancellationException) {
        if (interactive) {
          predictiveBackActive = false
          cancelPopDrag()
        }
        throw e
      }
      if (interactive) {
        predictiveBackActive = false
        animState = AnimState.PopGestureCommitted
        scope.launch { commitPopDrag() }
      } else {
        scope.launch { navigator.pop() }
      }
    }
    Box(
      modifier
        .fillMaxSize()
        .onSizeChanged {
          containerWidth = it.width.toFloat()
          containerHeight = it.height.toFloat()
        }
        .pointerInput(popNestedScroll) {
          awaitPointerEventScope {
            while (true) {
              val event = awaitPointerEvent(PointerEventPass.Initial)
              val pressedDragPointerCount =
                event.changes.count { change -> change.type.isTouchDragPointer() && change.pressed }
              val activePointer =
                if (pressedDragPointerCount == 1) {
                  event.changes.fastFirstOrNull { change ->
                    change.type.isTouchDragPointer() && change.pressed
                  }
                } else {
                  null
                }
              val releasedPointer =
                if (pressedDragPointerCount == 0) {
                  event.changes.fastFirstOrNull { change ->
                    change.type.isTouchDragPointer() && change.previousPressed && !change.pressed
                  }
                } else {
                  null
                }
              // Compose excludes the UP position from velocity samples, but still uses the
              // event to
              // apply its platform-specific pointer-stop timeout.
              val pointerSample = activePointer ?: releasedPointer
              popPointerVelocity.update(pressedDragPointerCount, pointerSample)
              popNestedScroll.updatePressedDragPointerCount(
                count = pressedDragPointerCount,
                downInSystemBackZone =
                  activePointer != null && activePointer.position.x < backGestureZoneWidth,
                pointerId = pointerSample?.id?.value,
                position = pointerSample?.position,
              )
            }
          }
        }
    ) {
      val useFadeTransition = transitionStyle == RouteTransitionStyle.Fade
      val useVerticalTransition = transitionStyle == RouteTransitionStyle.VerticalSlide
      val useSwitchTopBarTransition = useFadeTransition || useVerticalTransition

      when (animState) {
        AnimState.Idle -> topBarState.navDirection = NavDirection.Switch
        AnimState.Push ->
          topBarState.navDirection =
            if (useSwitchTopBarTransition) NavDirection.Switch else NavDirection.Push
        AnimState.Pop,
        AnimState.PopGestureCommitted ->
          topBarState.navDirection =
            if (useSwitchTopBarTransition) NavDirection.Switch else NavDirection.Pop
        AnimState.Dragging ->
          if (navigator.popRequested) {
            topBarState.navDirection =
              if (useSwitchTopBarTransition) NavDirection.Switch else NavDirection.Pop
          }
      }

      val mainRoute =
        when (animState) {
          // Push: 새 화면 (오른쪽에서 슬라이드 in)
          AnimState.Push -> navigator.current
          // Idle/Pop/Dragging: 현재 보이는 화면
          else -> visibleRoute
        }

      Box(
        Modifier.fillMaxSize()
          .graphicsLayer {
            alpha = 0f
            clip = true
          }
          .pointerInput(Unit) {
            awaitPointerEventScope {
              while (true) {
                awaitPointerEvent(PointerEventPass.Initial).changes.forEach { it.consume() }
              }
            }
          }
      ) {
        navigator.stack.forEach { route ->
          if (route == mainRoute || route == behindRoute) return@forEach
          if (!route.keepAlive) return@forEach
          Box(Modifier.fillMaxSize()) {
            presentRouteSurface(route, null, null)
            presentRouteForeground(route, null, null, Modifier.fillMaxSize())
          }
        }
      }

      Box(
        Modifier.fillMaxSize().pointerInput(navigationPopActivationDistance) {
          detectNavigationPopDrag(
            activationDistance = navigationPopActivationDistance,
            pointerVelocity = popPointerVelocity,
            canStart = ::canStartPopGesture,
            isSequenceRejected = { popNestedScroll.isCurrentSequenceRejected },
            onStart = ::startPopDrag,
            onDrag = ::updatePopDrag,
            onRelease = popNestedScroll::finishDirectGesture,
            onCancel = popNestedScroll::cancelDirectGesture,
          )
        }
      ) {
        val behindTopBar =
          when (animState) {
            AnimState.Push -> exitTopBarState
            // popRequested = commit 확정 → TopBar를 목적지로 전환
            AnimState.Dragging -> if (navigator.popRequested) topBarState else exitTopBarState
            else -> topBarState
          }
        val behindBottomBar =
          if (bottomBarState != null && exitBottomBarState != null) {
            when (animState) {
              AnimState.Push -> exitBottomBarState
              AnimState.Dragging ->
                if (navigator.popRequested) bottomBarState else exitBottomBarState
              else -> bottomBarState
            }
          } else {
            bottomBarState
          }
        val mainTopBar =
          when (animState) {
            // Pop: 나가는 화면은 exitTopBarState
            AnimState.Pop,
            AnimState.PopGestureCommitted -> exitTopBarState
            AnimState.Dragging -> if (navigator.popRequested) exitTopBarState else topBarState
            else -> topBarState
          }
        val mainBottomBar =
          if (bottomBarState != null && exitBottomBarState != null) {
            when (animState) {
              AnimState.Pop,
              AnimState.PopGestureCommitted -> exitBottomBarState
              AnimState.Dragging ->
                if (navigator.popRequested) exitBottomBarState else bottomBarState
              else -> bottomBarState
            }
          } else {
            bottomBarState
          }

        val p = progress.value
        val mainPresentation =
          when {
            animState == AnimState.Idle -> NavigationRoutePresentation()
            useFadeTransition ->
              NavigationRoutePresentation(
                alpha =
                  when (animState) {
                    AnimState.Push -> p
                    AnimState.Pop -> 1f - p
                    AnimState.PopGestureCommitted -> 1f
                    AnimState.Dragging -> if (navigator.popRequested) 1f - p else 1f
                    AnimState.Idle -> 1f
                  }
              )
            useVerticalTransition ->
              NavigationRoutePresentation(
                translationY =
                  when (animState) {
                    AnimState.Push -> containerHeight * (1f - p)
                    AnimState.Pop,
                    AnimState.PopGestureCommitted,
                    AnimState.Dragging -> containerHeight * p
                    AnimState.Idle -> 0f
                  },
                clipShape =
                  AppShapes.rounded(cornerRadius(if (animState == AnimState.Push) p else 1f - p)),
              )
            else ->
              NavigationRoutePresentation(
                translationX =
                  when (animState) {
                    AnimState.Push -> containerWidth * (1f - p)
                    else -> containerWidth * p
                  },
                clipShape =
                  AppShapes.rounded(cornerRadius(if (animState == AnimState.Push) p else 1f - p)),
              )
          }
        val behindPresentation =
          when {
            useFadeTransition ->
              NavigationRoutePresentation(
                alpha =
                  when (animState) {
                    AnimState.Push -> 1f - p
                    AnimState.Pop -> p
                    AnimState.PopGestureCommitted -> 0f
                    AnimState.Dragging -> if (navigator.popRequested) p else 0f
                    AnimState.Idle -> 1f
                  }
              )
            useVerticalTransition -> NavigationRoutePresentation()
            else ->
              NavigationRoutePresentation(
                translationX =
                  when (animState) {
                    AnimState.Push -> -containerWidth / 6f * p
                    else -> -containerWidth / 6f * (1f - p)
                  }
              )
          }
        val behindDimPresentation =
          when {
            useFadeTransition -> null
            useVerticalTransition ->
              NavigationRoutePresentation(
                alpha =
                  when (animState) {
                    AnimState.Push -> p
                    AnimState.Pop,
                    AnimState.PopGestureCommitted,
                    AnimState.Dragging -> 1f - p
                    AnimState.Idle -> 0f
                  }
              )
            else ->
              NavigationRoutePresentation(
                translationX = behindPresentation.translationX,
                alpha = if (animState == AnimState.Push) p else 1f - p,
              )
          }

        // Scene surfaces retain the current behind/main route order and are captured as one live
        // composite for the fixed top bar backdrop.
        Box(
          Modifier.fillMaxSize()
            .testTag(NavigationSceneSurfaceCompositeTestTag)
            .hazeSource(topBarBackdropHazeState)
        ) {
          behindRoute?.let { route ->
            Box(Modifier.fillMaxSize().navigationRoutePresentation(behindPresentation)) {
              presentRouteSurface(route, behindTopBar, behindBottomBar)
            }
            behindDimPresentation?.let { presentation ->
              Box(
                Modifier.fillMaxSize()
                  .navigationRoutePresentation(presentation)
                  .background(AppTheme.colors.scrim.copy(alpha = 0.5f))
                  .pointerIgnore()
              )
            }
          }
          Box(Modifier.fillMaxSize().navigationRoutePresentation(mainPresentation)) {
            presentRouteSurface(mainRoute, mainTopBar, mainBottomBar)
          }
        }

        val mainBackdropWeight =
          when {
            behindRoute == null -> 1f
            animState == AnimState.Push -> p
            animState == AnimState.Pop ||
              animState == AnimState.PopGestureCommitted ||
              animState == AnimState.Dragging -> 1f - p
            else -> 1f
          }
        val backdropStyle =
          resolveNavigationTopBarBackdropStyle(
            behindBackground =
              behindRoute?.let { route ->
                routeSceneFor(route).foregroundRegistry.topBarBackdropBackground
              },
            behindPresence = if (behindTopBar.hasTopBarBackdrop()) 1f else 0f,
            mainBackground = routeSceneFor(mainRoute).foregroundRegistry.topBarBackdropBackground,
            mainPresence = if (mainTopBar.hasTopBarBackdrop()) 1f else 0f,
            mainWeight = mainBackdropWeight,
            fallbackBackground = AppTheme.colors.surfaceCanvas,
          )
        NavigationTopBarBackdrop(
          hazeState = topBarBackdropHazeState,
          style = backdropStyle,
          modifier = Modifier.align(Alignment.TopCenter),
        )

        // Scene foregrounds use the matching presentation. Behind foreground is excluded from the
        // transformed main-route coverage so it cannot leak over an opaque front surface.
        behindRoute?.let { route ->
          presentRouteForeground(
            route,
            behindTopBar,
            behindBottomBar,
            Modifier.fillMaxSize()
              .excludeNavigationRouteCoverage(mainPresentation)
              .navigationRoutePresentation(behindPresentation),
          )
        }
        presentRouteForeground(
          mainRoute,
          mainTopBar,
          mainBottomBar,
          Modifier.fillMaxSize().navigationRoutePresentation(mainPresentation),
        )

        // 전환/드래그 중 route foreground까지 포함한 전체 presented scene의 터치를 차단한다.
        // 이미 시작된 direct drag의 hit path에는 이 노드가 없으므로 제스처 추적은 계속된다.
        if (animState != AnimState.Idle) {
          Box(Modifier.fillMaxSize().pointerIgnore())
        }

        // 엣지 제스처 감지 영역 (platform touch slop의 3배를 넘긴 dominant-right drag만 claim)
        if (navigator.canPop && (animState == AnimState.Idle || animState == AnimState.Dragging)) {
          Box(
            Modifier.fillMaxHeight().width(20.dp).align(Alignment.CenterStart).pointerInput(
              navigationPopActivationDistance
            ) {
              detectNavigationPopDrag(
                activationDistance = navigationPopActivationDistance,
                pointerVelocity = popPointerVelocity,
                canStart = ::canStartPopGesture,
                isSequenceRejected = { popNestedScroll.isCurrentSequenceRejected },
                onStart = ::startPopDrag,
                onDrag = ::updatePopDrag,
                onRelease = popNestedScroll::finishDirectGesture,
                onCancel = popNestedScroll::cancelDirectGesture,
              )
            }
          )
        }
      }
    }
  }
}

private fun cornerRadius(progress: Float): Dp {
  val maxRadius = AppShapes.xl
  val factor =
    when {
      progress < 0.95f -> 1f
      else -> (1f - (progress - 0.95f) / 0.05f)
    }
  return maxRadius * factor.coerceIn(0f, 1f)
}

private fun TopBarState?.hasTopBarBackdrop(): Boolean = this != null && enabled
