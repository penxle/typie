package co.typie.navigation

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.Spring.StiffnessMediumLow
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.ProvidedValue
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.ext.pointerIgnore
import co.typie.route.Route
import co.typie.route.RouteTransitionStyle
import co.typie.route.transitionStyleTo
import co.typie.ui.component.bottombar.BottomBarState
import co.typie.ui.component.bottombar.LocalBottomBarAnimationSource
import co.typie.ui.component.bottombar.LocalBottomBarState
import co.typie.ui.component.bottombar.ProvideBottomBar
import co.typie.ui.component.topbar.LocalTopBarState
import co.typie.ui.component.topbar.NavDirection
import co.typie.ui.component.topbar.TopBarState
import co.typie.ui.theme.AppTheme
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

  @Composable
  fun RouteContent(route: Route) {
    val owner =
      remember(route) {
        object : ViewModelStoreOwner {
          override val viewModelStore = navigator.viewModelStoreFor(route)
        }
      }
    CompositionLocalProvider(LocalViewModelStoreOwner provides owner, LocalRoute provides route) {
      ProvideBottomBar(enabled = false)
      content(route)
    }
  }

  val scope = rememberCoroutineScope()
  var containerWidth by remember { mutableStateOf(0f) }
  var animState by remember { mutableStateOf(AnimState.Idle) }

  // visibleRoute: Idle 상태에서 보이는 화면. 애니메이션 완료 후 업데이트.
  var visibleRoute by remember { mutableStateOf(navigator.current) }
  // behindRoute: 애니메이션/제스처 중 뒤에 깔리는 화면. Idle에서는 null.
  var behindRoute by remember { mutableStateOf<Route?>(null) }
  var transitionStyle by remember { mutableStateOf(RouteTransitionStyle.Slide) }

  val progress = remember { Animatable(0f) }

  // requestPop: 애니메이션 먼저, 그 다음 상태 변경
  LaunchedEffect(Unit) {
    snapshotFlow { navigator.popRequested }
      .collect { requested ->
        if (requested && animState == AnimState.Idle) {
          behindRoute = navigator.previous
          behindRoute?.let { transitionStyle = visibleRoute.transitionStyleTo(it) }
          animState = AnimState.Pop
          progress.snapTo(0f)
          progress.animateTo(1f, tween(350, easing = FastOutSlowInEasing))
          val poppedRoute = visibleRoute
          navigator.performPop()
          navigator.consumePopRequest()
          visibleRoute = navigator.current
          behindRoute = null
          animState = AnimState.Idle
          topBarState.clearRoute(poppedRoute)
          exitTopBarState.clearRoute(poppedRoute)
          bottomBarState?.clearRoute(poppedRoute)
          exitBottomBarState?.clearRoute(poppedRoute)
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
      bottomBarState?.let { add(LocalBottomBarAnimationSource provides it) }
    }
  CompositionLocalProvider(*animationProviders.toTypedArray()) {
    PlatformBackHandler(enabled = navigator.canPop) { scope.launch { navigator.pop() } }
    Box(modifier.fillMaxSize().onSizeChanged { containerWidth = it.width.toFloat() }) {
      val useFadeTransition = transitionStyle == RouteTransitionStyle.Fade

      when (animState) {
        AnimState.Idle -> topBarState.navDirection = NavDirection.Switch
        AnimState.Push ->
          topBarState.navDirection =
            if (useFadeTransition) NavDirection.Switch else NavDirection.Push
        AnimState.Pop ->
          topBarState.navDirection =
            if (useFadeTransition) NavDirection.Switch else NavDirection.Pop
        AnimState.Dragging ->
          if (navigator.popRequested) {
            topBarState.navDirection =
              if (useFadeTransition) NavDirection.Switch else NavDirection.Pop
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
            val behindProviders =
              buildList<ProvidedValue<*>> {
                add(LocalTopBarState provides behindTopBar)
                behindBottomBar?.let { add(LocalBottomBarState provides it) }
              }
            CompositionLocalProvider(*behindProviders.toTypedArray()) {
              RouteContent(behindRoute!!)
            }
          }
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
            val behindProviders =
              buildList<ProvidedValue<*>> {
                add(LocalTopBarState provides behindTopBar)
                behindBottomBar?.let { add(LocalBottomBarState provides it) }
              }
            CompositionLocalProvider(*behindProviders.toTypedArray()) {
              RouteContent(behindRoute!!)
            }
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
              .background(AppTheme.colors.shadow.copy(alpha = 0.5f))
              .pointerIgnore()
          )
        }
      }

      // Main layer (현재 화면 — 항상 같은 composition slot을 유지하여
      // Push→Idle 전환 시 remember 등 composition 상태가 보존됨)
      val mainRoute =
        when (animState) {
          // Push: 새 화면 (오른쪽에서 슬라이드 in)
          AnimState.Push -> navigator.current
          // Idle/Pop/Dragging: 현재 보이는 화면
          else -> visibleRoute
        }
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
        Modifier.fillMaxSize().graphicsLayer {
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
            } else {
              translationX =
                when (animState) {
                  // Push: 오른쪽에서 왼쪽으로 슬라이드 in
                  AnimState.Push -> containerWidth * (1f - p)
                  // Pop/Dragging: 오른쪽으로 슬라이드 out
                  else -> containerWidth * p
                }
              shadowElevation = 12.dp.toPx()
              shape =
                RoundedCornerShape(
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
        val mainProviders =
          buildList<ProvidedValue<*>> {
            add(LocalTopBarState provides mainTopBar)
            mainBottomBar?.let { add(LocalBottomBarState provides it) }
          }
        CompositionLocalProvider(*mainProviders.toTypedArray()) { RouteContent(mainRoute) }
      }

      // 제스처 감지 영역
      if (navigator.canPop && (animState == AnimState.Idle || animState == AnimState.Dragging)) {
        var lastDragAmount by remember { mutableStateOf(0f) }
        Box(
          Modifier.fillMaxHeight().width(20.dp).align(Alignment.CenterStart).pointerInput(Unit) {
            detectHorizontalDragGestures(
              onDragStart = {
                navigator.previous?.let {
                  transitionStyle = visibleRoute.transitionStyleTo(it)
                  behindRoute = it
                  animState = AnimState.Dragging
                  scope.launch { progress.snapTo(0f) }
                }
              },
              onDragEnd = {
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
              },
              onDragCancel = {
                scope.launch {
                  progress.animateTo(0f, spring(stiffness = StiffnessMediumLow))
                  behindRoute = null
                  animState = AnimState.Idle
                }
              },
              onHorizontalDrag = { _, dragAmount ->
                lastDragAmount = dragAmount
                scope.launch {
                  val newValue = (progress.value + dragAmount / containerWidth).coerceIn(0f, 1f)
                  progress.snapTo(newValue)
                }
              },
            )
          }
        )
      }

      // Modals
      navigator.modals.forEach { modalContent -> modalContent() }
    }
  }
}

private fun cornerRadius(progress: Float): Dp {
  val maxRadius = 24.dp
  val factor =
    when {
      progress < 0.95f -> 1f
      else -> (1f - (progress - 0.95f) / 0.05f)
    }
  return maxRadius * factor.coerceIn(0f, 1f)
}
