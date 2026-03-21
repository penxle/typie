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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.route.Route
import co.typie.ui.theme.AppTheme
import androidx.compose.runtime.snapshotFlow
import kotlinx.coroutines.launch

private enum class AnimState { Idle, Push, Pop, Dragging }

@Composable
fun NavigationStack(
  navigator: Navigator,
  modifier: Modifier = Modifier,
  content: @Composable (Route) -> Unit,
) {
  @Composable
  fun RouteContent(route: Route) {
    val owner = remember(route) {
      object : ViewModelStoreOwner {
        override val viewModelStore = navigator.viewModelStoreFor(route)
      }
    }
    CompositionLocalProvider(LocalViewModelStoreOwner provides owner) {
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

  val progress = remember { Animatable(0f) }

  // requestPop: 애니메이션 먼저, 그 다음 상태 변경
  LaunchedEffect(Unit) {
    snapshotFlow { navigator.popRequested }
      .collect { requested ->
        if (requested && animState == AnimState.Idle) {
          behindRoute = navigator.previous
          animState = AnimState.Pop
          progress.snapTo(0f)
          progress.animateTo(1f, tween(350, easing = FastOutSlowInEasing))
          navigator.pop()
          navigator.consumePopRequest()
          visibleRoute = navigator.current
          behindRoute = null
          animState = AnimState.Idle
        }
      }
  }

  // Push 및 직접 pop() 호출 처리
  LaunchedEffect(navigator.current) {
    if (navigator.current != visibleRoute) {
      when (navigator.lastOperation) {
        NavOperation.Push -> {
          // Push: visibleRoute(이전 화면)가 뒤로, navigator.current(새 화면)가 앞으로
          behindRoute = visibleRoute
          animState = AnimState.Push
          progress.snapTo(0f)
          progress.animateTo(1f, tween(350, easing = FastOutSlowInEasing))
          visibleRoute = navigator.current
        }

        else -> {
          // Pop: visibleRoute(현재 화면)가 앞에서 나가고, navigator.current(이전 화면)가 뒤에서 나타남
          behindRoute = navigator.current
          animState = AnimState.Pop
          progress.snapTo(0f)
          progress.animateTo(1f, tween(350, easing = FastOutSlowInEasing))
          visibleRoute = navigator.current
        }
      }
      behindRoute = null
      animState = AnimState.Idle
    }
  }

  CompositionLocalProvider(Nav provides navigator) {
    PlatformBackHandler(enabled = navigator.canPop) {
      navigator.requestPop()
    }
    Box(
      modifier
        .fillMaxSize()
        .onSizeChanged { containerWidth = it.width.toFloat() }
    ) {
      when (animState) {
        AnimState.Idle -> {
          RouteContent(visibleRoute)
        }

        AnimState.Push -> {
          val p = progress.value
          val behindOffset = -containerWidth / 6f * p
          val frontOffset = containerWidth * (1f - p)

          // 뒤: 이전 화면 (왼쪽으로 밀림)
          Box(Modifier.fillMaxSize().graphicsLayer {
            translationX = behindOffset
          }) { RouteContent(behindRoute!!) }

          // Dim overlay
          Box(Modifier.fillMaxSize().graphicsLayer { translationX = behindOffset }
            .background(AppTheme.colors.shadowDefault.copy(alpha = 0.5f * p)))

          // 앞: 새 화면 (오른쪽에서 슬라이드 in)
          Box(Modifier.fillMaxSize().graphicsLayer {
            translationX = frontOffset
            shadowElevation = 12.dp.toPx()
            shape = RoundedCornerShape(cornerRadius(p))
            clip = true
          }) { RouteContent(navigator.current) }
        }

        AnimState.Pop -> {
          val p = progress.value
          val behindOffset = -containerWidth / 6f * (1f - p)
          val frontOffset = containerWidth * p

          // 뒤: 돌아갈 화면 (왼쪽에서 복귀)
          behindRoute?.let { behind ->
            Box(Modifier.fillMaxSize().graphicsLayer {
              translationX = behindOffset
            }) { RouteContent(behind) }

            // Dim overlay
            Box(Modifier.fillMaxSize().graphicsLayer { translationX = behindOffset }
              .background(AppTheme.colors.shadowDefault.copy(alpha = 0.5f * (1f - p))))

            // 앞: 나가는 화면 (오른쪽으로 슬라이드 out)
            Box(Modifier.fillMaxSize().graphicsLayer {
              translationX = frontOffset
              shadowElevation = 12.dp.toPx()
              shape = RoundedCornerShape(cornerRadius(1f - p))
              clip = true
            }) { RouteContent(visibleRoute) }
          }
        }

        AnimState.Dragging -> {
          val p = progress.value
          val behindOffset = -containerWidth / 6f * (1f - p)
          val frontOffset = containerWidth * p

          // 뒤: 돌아갈 화면
          behindRoute?.let { behind ->
            Box(Modifier.fillMaxSize().graphicsLayer {
              translationX = behindOffset
            }) { RouteContent(behind) }

            // Dim overlay
            Box(Modifier.fillMaxSize().graphicsLayer { translationX = behindOffset }
              .background(AppTheme.colors.shadowDefault.copy(alpha = 0.5f * (1f - p))))

            // 앞: 손가락 따라 움직이는 현재 화면
            Box(Modifier.fillMaxSize().graphicsLayer {
              translationX = frontOffset
              shadowElevation = 12.dp.toPx()
              shape = RoundedCornerShape(cornerRadius(1f - p))
              clip = true
            }) { RouteContent(visibleRoute) }
          }
        }
      }

      // 제스처 감지 영역
      if (navigator.canPop && (animState == AnimState.Idle || animState == AnimState.Dragging)) {
        var lastDragAmount by remember { mutableStateOf(0f) }
        Box(
          Modifier
            .fillMaxHeight()
            .width(20.dp)
            .align(Alignment.CenterStart)
            .pointerInput(Unit) {
              detectHorizontalDragGestures(
                onDragStart = {
                  behindRoute = navigator.previous
                  animState = AnimState.Dragging
                  scope.launch { progress.snapTo(0f) }
                },
                onDragEnd = {
                  val velocity = lastDragAmount * 1000f / 16f
                  scope.launch {
                    if (progress.value > 0.5f || velocity > 1000f) {
                      navigator.requestPop()
                      progress.animateTo(1f, spring(stiffness = StiffnessMediumLow))
                      navigator.pop()
                      navigator.consumePopRequest()
                      visibleRoute = navigator.current
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
            },
        )
      }

      // Modals
      navigator.modals.forEach { modalContent ->
        modalContent()
      }
    }
  }
}

private fun cornerRadius(progress: Float): Dp {
  val maxRadius = 24.dp
  val factor = when {
    progress < 0.95f -> 1f
    else -> (1f - (progress - 0.95f) / 0.05f)
  }
  return maxRadius * factor.coerceIn(0f, 1f)
}
