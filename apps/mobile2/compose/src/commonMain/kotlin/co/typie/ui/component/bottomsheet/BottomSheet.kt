package co.typie.ui.component.bottomsheet

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.background
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.draggable
import androidx.compose.foundation.gestures.rememberDraggableState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import co.typie.ext.clickable
import co.typie.ext.imePadding
import co.typie.ext.navigationBarsPadding
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

@Composable
fun BottomSheetHost(state: BottomSheetHostState) {
  for (entry in state.entries) {
    @Suppress("UNCHECKED_CAST")
    BottomSheetOverlay(entry = entry as BottomSheetEntry<Any?>)
  }
}

@Composable
private fun <T> BottomSheetOverlay(entry: BottomSheetEntry<T>) {
  val viewModelStore = remember { ViewModelStore() }
  val viewModelStoreOwner = remember {
    object : ViewModelStoreOwner {
      override val viewModelStore get() = viewModelStore
    }
  }
  DisposableEffect(Unit) {
    onDispose { viewModelStore.clear() }
  }

  val coroutineScope = rememberCoroutineScope()

  // 0f = hidden, 1f = expanded
  val progress = remember { Animatable(0f) }
  var isDismissing by remember { mutableStateOf(false) }
  var sheetHeightPx by remember { mutableFloatStateOf(0f) }

  // 드래그 중 추가 offset (px, 아래 방향 양수)
  var dragOffsetPx by remember { mutableFloatStateOf(0f) }
  val scrollState = rememberScrollState()

  // 스크롤 최상단에서 아래로 overscroll → 시트 드래그로 전환
  val nestedScrollConnection = remember {
    object : NestedScrollConnection {
      override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
        if (available.y > 0f && scrollState.value == 0) {
          dragOffsetPx = (dragOffsetPx + available.y).coerceAtLeast(0f)
          return Offset(0f, available.y)
        }
        return Offset.Zero
      }
    }
  }

  val enterSpec = tween<Float>(BottomSheetDefaults.EnterDuration, easing = BottomSheetDefaults.EnterEasing)
  val exitSpec = tween<Float>(BottomSheetDefaults.ExitDuration, easing = BottomSheetDefaults.ExitEasing)

  LaunchedEffect(Unit) {
    progress.animateTo(1f, enterSpec)
  }

  fun dismissWithResult(result: T) {
    if (isDismissing) return
    isDismissing = true
    coroutineScope.launch {
      progress.animateTo(0f, exitSpec)
      entry.resume(result)
    }
  }

  fun dismissWithoutResult() {
    if (isDismissing) return
    isDismissing = true
    coroutineScope.launch {
      progress.animateTo(0f, exitSpec)
      entry.cancel()
    }
  }

  val sheetScope = remember(entry) {
    object : BottomSheetScope<T> {
      override fun dismiss(result: T) {
        dismissWithResult(result)
      }
    }
  }

  PlatformBackHandler(enabled = !isDismissing) {
    dismissWithoutResult()
  }

  // 스와이프 제스처
  var dragDebt by remember { mutableFloatStateOf(0f) }
  val draggableState = rememberDraggableState { delta ->
    when {
      dragOffsetPx > 0f -> {
        // 시트가 내려간 상태 — 위/아래 모두 시트 이동
        val newOffset = dragOffsetPx + delta
        if (newOffset < 0f) {
          dragOffsetPx = 0f
          dragDebt = -newOffset
        } else {
          dragOffsetPx = newOffset
        }
      }
      delta < 0f -> {
        // 원위치에서 위로 — 빚 누적
        dragDebt -= delta
      }
      dragDebt > 0f -> {
        // 빚 상환 중
        dragDebt = (dragDebt - delta).coerceAtLeast(0f)
      }
      else -> {
        // 아래로 — 시트 이동
        dragOffsetPx = (dragOffsetPx + delta).coerceAtLeast(0f)
      }
    }
  }

  val measured = sheetHeightPx > 0f

  // 총 offset = 애니메이션 offset + 드래그 offset
  val animOffsetPx = if (measured) (1f - progress.value) * sheetHeightPx else 0f
  val totalOffsetY = if (measured) (animOffsetPx + dragOffsetPx).roundToInt() else 9999

  // scrim alpha는 시트 가시성에 비례
  val visibility = if (measured && sheetHeightPx > 0f) {
    (1f - (animOffsetPx + dragOffsetPx) / sheetHeightPx).coerceIn(0f, 1f)
  } else 0f
  val scrimAlpha = visibility * BottomSheetDefaults.ScrimAlpha

  Box(
    modifier = Modifier.fillMaxSize(),
    contentAlignment = Alignment.BottomCenter,
  ) {
    // Scrim
    Box(
      Modifier
        .fillMaxSize()
        .alpha(scrimAlpha)
        .background(AppTheme.colors.scrim)
        .clickable { dismissWithoutResult() },
    )

    // Sheet surface
    val colors = AppTheme.colors
    BoxWithConstraints(Modifier.fillMaxWidth()) {
      val maxSheetHeight = maxHeight * BottomSheetDefaults.MaxHeightFraction

      Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier
          .fillMaxWidth()
          .heightIn(max = maxSheetHeight)
          .onSizeChanged { sheetHeightPx = it.height.toFloat() }
          .offset { IntOffset(0, totalOffsetY) }
          .dropShadow(RoundedCornerShape(topStart = BottomSheetDefaults.TopCornerRadius, topEnd = BottomSheetDefaults.TopCornerRadius)) {
            color = colors.shadowAmbient
            radius = 8f
          }
          .dropShadow(RoundedCornerShape(topStart = BottomSheetDefaults.TopCornerRadius, topEnd = BottomSheetDefaults.TopCornerRadius)) {
            color = colors.shadow
            offset = Offset(0f, -4f)
            radius = 12f
          }
          .draggable(
          state = draggableState,
          orientation = Orientation.Vertical,
          onDragStopped = { velocity ->
            dragDebt = 0f
            if (dragOffsetPx > sheetHeightPx * 0.3f || velocity > 1000f) {
              // dismiss
              dismissWithoutResult()
            } else {
              // snap back
              coroutineScope.launch {
                val snapBack = Animatable(dragOffsetPx)
                snapBack.animateTo(0f, enterSpec) { dragOffsetPx = value }
              }
            }
          },
        )
        .clip(RoundedCornerShape(topStart = BottomSheetDefaults.TopCornerRadius, topEnd = BottomSheetDefaults.TopCornerRadius))
        .background(colors.surfaceRaised)
        .navigationBarsPadding()
        .imePadding(),
    ) {
      // Handle
      Spacer(modifier = Modifier.height(BottomSheetDefaults.HandleTopPadding))
      Box(
        modifier = Modifier
          .size(width = BottomSheetDefaults.HandleWidth, height = BottomSheetDefaults.HandleHeight)
          .clip(RoundedCornerShape(BottomSheetDefaults.HandleHeight / 2))
          .background(colors.borderSubtle),
      )
      Spacer(modifier = Modifier.height(BottomSheetDefaults.HandleTopPadding))

      // Content
      Box(
        modifier = Modifier
          .fillMaxWidth()
          .weight(1f, fill = false)
          .nestedScroll(nestedScrollConnection)
          .verticalScroll(scrollState),
      ) {
        CompositionLocalProvider(LocalViewModelStoreOwner provides viewModelStoreOwner) {
          entry.content.invoke(sheetScope)
        }
      }
    }
    }
  }
}
