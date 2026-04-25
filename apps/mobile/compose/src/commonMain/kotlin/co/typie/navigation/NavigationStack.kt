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
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChangeIgnoreConsumed
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.util.fastFirstOrNull
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.ext.pointerIgnore
import co.typie.ext.thenIf
import co.typie.route.Route
import co.typie.route.RouteTransitionStyle
import co.typie.route.keepAlive
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
import kotlin.math.abs
import kotlinx.coroutines.launch

private enum class AnimState {
  Idle,
  Push,
  Pop,
  Dragging,
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
  val routeContents = remember {
    mutableMapOf<Route, @Composable (TopBarState?, BottomBarState?) -> Unit>()
  }
  val routeContentFor: (Route) -> @Composable (TopBarState?, BottomBarState?) -> Unit = { route ->
    routeContents.getOrPut(route) {
      movableContentOf<TopBarState?, BottomBarState?> { topBar, bottomBar ->
        val owner = remember {
          object : ViewModelStoreOwner {
            override val viewModelStore = navigator.viewModelStoreFor(route)
          }
        }
        val providers =
          buildList<ProvidedValue<*>> {
            add(LocalViewModelStoreOwner provides owner)
            add(LocalRoute provides route)
            add(LocalTopBarState provides topBar)
            if (bottomBar != null) add(LocalBottomBarState provides bottomBar)
          }
        CompositionLocalProvider(*providers.toTypedArray()) {
          ProvideBottomBar(enabled = false)
          latestContent(route)
        }
      }
    }
  }

  // 백스택에서 제거된 라우트의 movable 캐시 정리
  LaunchedEffect(Unit) {
    snapshotFlow { navigator.stack.toSet() }
      .collect { active -> routeContents.keys.retainAll(active) }
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

  var lastDragAmount by remember { mutableStateOf(0f) }

  fun startPopDrag() {
    val prev = navigator.previous ?: return
    transitionStyle = visibleRoute.transitionStyleTo(prev)
    behindRoute = prev
    animState = AnimState.Dragging
    scope.launch { progress.snapTo(0f) }
  }

  fun updatePopDrag(dragAmount: Float) {
    lastDragAmount = dragAmount
    scope.launch {
      val newValue = (progress.value + dragAmount / containerWidth).coerceIn(0f, 1f)
      progress.snapTo(newValue)
    }
  }

  fun finishPopDrag() {
    val velocity = lastDragAmount * 1000f / 16f
    scope.launch {
      if (progress.value > 0.5f || velocity > 1000f) {
        val poppedRoute = visibleRoute
        navigator.requestPop()
        progress.animateTo(1f, spring(stiffness = StiffnessMediumLow))
        navigator.performPop()
        navigator.consumePopRequest()
        visibleRoute = navigator.current
        topBarState.clearRoute(poppedRoute)
        exitTopBarState.clearRoute(poppedRoute)
        bottomBarState?.clearRoute(poppedRoute)
        exitBottomBarState?.clearRoute(poppedRoute)
      } else {
        progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
      }
      behindRoute = null
      animState = AnimState.Idle
    }
  }

  fun cancelPopDrag() {
    scope.launch {
      progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
      behindRoute = null
      animState = AnimState.Idle
    }
  }

  // requestPop: 애니메이션 먼저, 그 다음 상태 변경
  LaunchedEffect(Unit) {
    snapshotFlow { navigator.popRequested }
      .collect { requested ->
        if (requested && animState == AnimState.Idle) {
          val popTarget = navigator.peekPopTarget()
          val targetRoute = popTarget ?: navigator.previous
          if (targetRoute == null) {
            navigator.consumePopRequest()
            navigator.completeTransition()
            return@collect
          }

          behindRoute = targetRoute
          transitionStyle = visibleRoute.transitionStyleTo(targetRoute)
          animState = AnimState.Pop
          progress.snapTo(0f)
          progress.animateTo(1f, tween(350, easing = FastOutSlowInEasing))
          val removedRoutes =
            if (popTarget != null) {
              navigator.performPopTo(popTarget)
            } else {
              navigator.performPop()
              listOf(visibleRoute)
            }
          navigator.consumePopRequest()
          visibleRoute = navigator.current
          behindRoute = null
          animState = AnimState.Idle
          removedRoutes.forEach { removedRoute ->
            topBarState.clearRoute(removedRoute)
            exitTopBarState.clearRoute(removedRoute)
            bottomBarState?.clearRoute(removedRoute)
            exitBottomBarState?.clearRoute(removedRoute)
          }
          navigator.completeTransition()
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
          topBarState.clearRoute(poppedRoute)
          exitTopBarState.clearRoute(poppedRoute)
          bottomBarState?.clearRoute(poppedRoute)
          exitBottomBarState?.clearRoute(poppedRoute)
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
      add(LocalTopBarAnimationSource provides topBarState)
      bottomBarState?.let { add(LocalBottomBarAnimationSource provides it) }
    }
  CompositionLocalProvider(*animationProviders.toTypedArray()) {
    PlatformBackHandler(enabled = navigator.canPop) { scope.launch { navigator.pop() } }
    Box(
      modifier.fillMaxSize().onSizeChanged {
        containerWidth = it.width.toFloat()
        containerHeight = it.height.toFloat()
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
        AnimState.Pop ->
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
          Box(Modifier.fillMaxSize()) { routeContentFor(route).invoke(null, null) }
        }
      }

      // Behind layer (애니메이션 중 뒤에 깔리는 화면)
      if (behindRoute != null) {
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

        if (useFadeTransition) {
          Box(
            Modifier.fillMaxSize().graphicsLayer {
              alpha =
                when (animState) {
                  AnimState.Push -> 1f - progress.value
                  AnimState.Pop -> progress.value
                  AnimState.Dragging -> if (navigator.popRequested) progress.value else 0f
                  AnimState.Idle -> 1f
                }
            }
          ) {
            routeContentFor(behindRoute!!).invoke(behindTopBar, behindBottomBar)
          }
          // 전환 중 behind 화면 터치 차단 (fade는 dim overlay가 없으므로 별도 pointerIgnore)
          Box(Modifier.fillMaxSize().pointerIgnore())
        } else if (useVerticalTransition) {
          Box(Modifier.fillMaxSize()) {
            routeContentFor(behindRoute!!).invoke(behindTopBar, behindBottomBar)
          }
          // Dim overlay — 전환 중 behind 화면 터치 차단
          Box(
            Modifier.fillMaxSize()
              .graphicsLayer {
                alpha =
                  when (animState) {
                    AnimState.Push -> progress.value
                    AnimState.Pop -> 1f - progress.value
                    AnimState.Dragging -> 1f - progress.value
                    AnimState.Idle -> 0f
                  }
              }
              .background(AppTheme.colors.scrim.copy(alpha = 0.5f))
              .pointerIgnore()
          )
        } else {
          Box(
            Modifier.fillMaxSize().graphicsLayer {
              translationX =
                when (animState) {
                  // Push: 이전 화면이 왼쪽으로 밀림
                  AnimState.Push -> -containerWidth / 6f * progress.value
                  // Pop/Dragging: 돌아갈 화면이 왼쪽에서 복귀
                  else -> -containerWidth / 6f * (1f - progress.value)
                }
            }
          ) {
            routeContentFor(behindRoute!!).invoke(behindTopBar, behindBottomBar)
          }
          // Dim overlay — 전환 중 behind 화면 터치 차단
          Box(
            Modifier.fillMaxSize()
              .graphicsLayer {
                val p = progress.value
                translationX =
                  when (animState) {
                    AnimState.Push -> -containerWidth / 6f * p
                    else -> -containerWidth / 6f * (1f - p)
                  }
                alpha =
                  when (animState) {
                    AnimState.Push -> p
                    else -> 1f - p
                  }
              }
              .background(AppTheme.colors.scrim.copy(alpha = 0.5f))
              .pointerIgnore()
          )
        }
      }

      // Main layer (현재 화면 — 항상 같은 composition slot을 유지하여
      // Push→Idle 전환 시 remember 등 composition 상태가 보존됨)
      val mainTopBar =
        when (animState) {
          // Pop: 나가는 화면은 exitTopBarState
          AnimState.Pop -> exitTopBarState
          AnimState.Dragging -> if (navigator.popRequested) exitTopBarState else topBarState
          else -> topBarState
        }
      val mainBottomBar =
        if (bottomBarState != null && exitBottomBarState != null) {
          when (animState) {
            AnimState.Pop -> exitBottomBarState
            AnimState.Dragging -> if (navigator.popRequested) exitBottomBarState else bottomBarState
            else -> bottomBarState
          }
        } else {
          bottomBarState
        }

      Box(
        Modifier.fillMaxSize()
          .thenIf(navigator.canPop) {
            pointerInput(Unit) {
              val slop = viewConfiguration.touchSlop
              awaitEachGesture {
                val down = awaitFirstDown(requireUnconsumed = false)
                if (animState != AnimState.Idle && animState != AnimState.Dragging)
                  return@awaitEachGesture
                var overSlopX = 0f
                var claimed = false
                while (!claimed) {
                  val event = awaitPointerEvent(PointerEventPass.Main)
                  val change =
                    event.changes.fastFirstOrNull { it.id == down.id } ?: return@awaitEachGesture
                  if (!change.pressed) return@awaitEachGesture
                  val dx = change.position.x - down.position.x
                  val dy = change.position.y - down.position.y
                  if (abs(dx) > slop || abs(dy) > slop) {
                    if (dx <= 0f || abs(dx) <= abs(dy)) return@awaitEachGesture
                    if (change.isConsumed) return@awaitEachGesture
                    val confirmEvent = awaitPointerEvent(PointerEventPass.Main)
                    val confirmChange =
                      confirmEvent.changes.fastFirstOrNull { it.id == down.id }
                        ?: return@awaitEachGesture
                    if (!confirmChange.pressed) return@awaitEachGesture
                    if (confirmChange.isConsumed) return@awaitEachGesture
                    confirmChange.consume()
                    overSlopX = confirmChange.position.x - down.position.x
                    claimed = true
                  }
                }
                startPopDrag()
                updatePopDrag(overSlopX)
                // blocker가 Main pass 전에 consume하므로 isConsumed를 무시하고 loop.
                // 자식 scrollable이 이 포인터를 take over하지 못하도록 우리가 계속 consume.
                var dragging = true
                while (dragging) {
                  val event = awaitPointerEvent(PointerEventPass.Main)
                  val change = event.changes.fastFirstOrNull { it.id == down.id }
                  if (change == null) {
                    dragging = false
                    continue
                  }
                  updatePopDrag(change.positionChangeIgnoreConsumed().x)
                  if (!change.pressed) dragging = false else change.consume()
                }
                finishPopDrag()
              }
            }
          }
          .graphicsLayer {
            if (animState != AnimState.Idle) {
              val p = progress.value
              if (useFadeTransition) {
                alpha =
                  when (animState) {
                    AnimState.Push -> p
                    AnimState.Pop -> 1f - p
                    AnimState.Dragging -> if (navigator.popRequested) 1f - p else 1f
                    AnimState.Idle -> 1f
                  }
              } else if (useVerticalTransition) {
                translationY =
                  when (animState) {
                    AnimState.Push -> containerHeight * (1f - p)
                    AnimState.Pop -> containerHeight * p
                    AnimState.Dragging -> containerHeight * p
                    AnimState.Idle -> 0f
                  }
                shape =
                  AppShapes.rounded(
                    cornerRadius(
                      when (animState) {
                        AnimState.Push -> p
                        else -> 1f - p
                      }
                    )
                  )
                clip = true
              } else {
                translationX =
                  when (animState) {
                    // Push: 오른쪽에서 왼쪽으로 슬라이드 in
                    AnimState.Push -> containerWidth * (1f - p)
                    // Pop/Dragging: 오른쪽으로 슬라이드 out
                    else -> containerWidth * p
                  }
                shape =
                  AppShapes.rounded(
                    cornerRadius(
                      when (animState) {
                        AnimState.Push -> p
                        else -> 1f - p
                      }
                    )
                  )
                clip = true
              }
            }
          }
      ) {
        routeContentFor(mainRoute).invoke(mainTopBar, mainBottomBar)
        // 전환/드래그 중 front 화면 내부로 터치가 흘러가지 않도록 consume.
        // Main pass 기준 overlay가 sibling routeContent보다 나중에 composed → 먼저 처리되어 consume.
        // Drag 로직은 Main pass에서 positionChangeIgnoreConsumed로 계속 추적하므로 영향받지 않는다.
        if (animState != AnimState.Idle) {
          Box(Modifier.fillMaxSize().pointerIgnore())
        }
      }

      // 엣지 제스처 감지 영역 (child consume 여부 무관하게 pop 우선)
      if (navigator.canPop && (animState == AnimState.Idle || animState == AnimState.Dragging)) {
        Box(
          Modifier.fillMaxHeight().width(20.dp).align(Alignment.CenterStart).pointerInput(Unit) {
            val slop = viewConfiguration.touchSlop
            awaitEachGesture {
              val down = awaitFirstDown(requireUnconsumed = false)
              var overSlopX = 0f
              var claimed = false
              while (!claimed) {
                val event = awaitPointerEvent(PointerEventPass.Main)
                val change =
                  event.changes.fastFirstOrNull { it.id == down.id } ?: return@awaitEachGesture
                if (!change.pressed) return@awaitEachGesture
                if (change.isConsumed) return@awaitEachGesture
                val dx = change.position.x - down.position.x
                if (abs(dx) >= slop) {
                  change.consume()
                  overSlopX = dx
                  claimed = true
                }
              }
              startPopDrag()
              updatePopDrag(overSlopX)
              var dragging = true
              while (dragging) {
                val event = awaitPointerEvent(PointerEventPass.Main)
                val change = event.changes.fastFirstOrNull { it.id == down.id }
                if (change == null) {
                  dragging = false
                  continue
                }
                updatePopDrag(change.positionChangeIgnoreConsumed().x)
                if (!change.pressed) dragging = false else change.consume()
              }
              finishPopDrag()
            }
          }
        )
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
