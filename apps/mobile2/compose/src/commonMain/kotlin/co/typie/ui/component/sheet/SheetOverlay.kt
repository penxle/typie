package co.typie.ui.component.sheet

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.AnchoredDraggableState
import androidx.compose.foundation.gestures.DraggableAnchors
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.anchoredDraggable
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.ext.clickable
import co.typie.ext.safeDrawing
import co.typie.ext.thenIf
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch

private const val ANCHOR_HIDDEN = -1
private const val ANCHOR_VISIBLE = 0
private val DefaultIntrinsicTopGap = 64.dp
private val SheetAnimationSpec = spring<Float>(stiffness = 500f)

@Composable
fun SheetOverlay(state: Sheet) {
  for (entry in state.entries) {
    key(entry) {
      SheetEntryOverlay(entry = entry, onResolve = { result -> state.resolveEntry(entry, result) })
    }
  }
}

@Composable
private fun SheetEntryOverlay(entry: SheetEntry<*>, onResolve: (Any?) -> Unit) {
  @Suppress("UNCHECKED_CAST") val typedEntry = entry as SheetEntry<Any?>

  val viewModelStore = remember { ViewModelStore() }
  val viewModelStoreOwner = remember {
    object : ViewModelStoreOwner {
      override val viewModelStore
        get() = viewModelStore
    }
  }
  DisposableEffect(Unit) { onDispose { viewModelStore.clear() } }

  var pendingResult by remember(entry) { mutableStateOf<Any?>(null) }
  var resolved by remember(entry) { mutableStateOf(false) }
  var dismissed by remember(entry) { mutableStateOf(false) }
  val hapticFeedback = LocalHapticFeedback.current
  val hapticFeedbackState = rememberUpdatedState(hapticFeedback)

  val handleDismissed: () -> Unit = {
    if (!dismissed) {
      dismissed = true
      onResolve(if (resolved) pendingResult else null)
    }
  }

  BoxWithConstraints(Modifier.fillMaxSize()) {
    val density = LocalDensity.current
    val containerHeightPx = with(density) { maxHeight.toPx() }
    val intrinsicTopLimitPx =
      with(density) {
        maxOf(WindowInsets.safeDrawing.getTop(density).toFloat(), DefaultIntrinsicTopGap.toPx())
      }
    val isIntrinsic = entry.stops.isEmpty()
    var contentHeightPx by remember { mutableStateOf(0f) }
    val coroutineScope = rememberCoroutineScope()
    val dragOverscrollEffect = remember { SheetTopHysteresisOverscrollEffect() }

    val visibleOffsets: List<Float> =
      remember(entry.stops, containerHeightPx, contentHeightPx, intrinsicTopLimitPx) {
        if (isIntrinsic) {
          if (contentHeightPx > 0f) {
            listOf(maxOf(containerHeightPx - contentHeightPx, intrinsicTopLimitPx))
          } else {
            emptyList()
          }
        } else {
          entry.stops.map { stop ->
            when (stop) {
              is SheetStop.Bottom -> containerHeightPx - with(density) { stop.height.toPx() }
              is SheetStop.Top -> with(density) { stop.margin.toPx() }
            }
          }
        }
      }

    val anchors =
      remember(visibleOffsets, containerHeightPx) {
        DraggableAnchors {
          visibleOffsets.forEachIndexed { index, offset -> index at offset }
          ANCHOR_HIDDEN at containerHeightPx
        }
      }

    val anchoredState = remember {
      AnchoredDraggableState(initialValue = ANCHOR_HIDDEN, anchors = anchors)
    }

    val offsetCorrection = remember { Animatable(0f) }

    LaunchedEffect(anchors) {
      val prevOffset = anchoredState.offset
      anchoredState.updateAnchors(anchors, anchoredState.targetValue)
      val newOffset = anchoredState.offset

      if (
        !isIntrinsic &&
          !prevOffset.isNaN() &&
          !newOffset.isNaN() &&
          prevOffset != newOffset &&
          anchoredState.currentValue != ANCHOR_HIDDEN
      ) {
        offsetCorrection.snapTo(prevOffset - newOffset)
        offsetCorrection.animateTo(0f, SheetAnimationSpec)
      }
    }

    LaunchedEffect(visibleOffsets.isNotEmpty()) {
      if (visibleOffsets.isEmpty()) return@LaunchedEffect

      anchoredState.animateTo(ANCHOR_VISIBLE, SheetAnimationSpec)

      snapshotFlow { anchoredState.settledValue }
        .filter { it == ANCHOR_HIDDEN }
        .collect { handleDismissed() }
    }

    LaunchedEffect(anchoredState) {
      var previousVisibleStop: Int? = null

      snapshotFlow { anchoredState.settledValue }
        .collect { settledValue ->
          val nextVisibleStop = settledValue.takeIf { it != ANCHOR_HIDDEN }
          if (
            previousVisibleStop != null &&
              nextVisibleStop != null &&
              nextVisibleStop != previousVisibleStop
          ) {
            hapticFeedbackState.value.performHapticFeedback(HapticFeedbackType.SegmentTick)
          }
          if (nextVisibleStop != null) {
            previousVisibleStop = nextVisibleStop
          }
        }
    }

    val requestDismiss: () -> Unit = {
      if (!dismissed) {
        coroutineScope.launch {
          anchoredState.animateTo(ANCHOR_HIDDEN, SheetAnimationSpec)
          handleDismissed()
        }
      }
    }

    val scope =
      remember(entry) {
        object : SheetScope<Any?> {
          override fun complete(result: Any?) {
            pendingResult = result
            resolved = true
            requestDismiss()
          }

          override fun dismiss() {
            requestDismiss()
          }
        }
      }

    val nestedScrollConnection =
      rememberSheetNestedScrollConnection(
        anchoredState = anchoredState,
        visibleOffsets = visibleOffsets,
        containerHeightPx = containerHeightPx,
        hiddenValue = ANCHOR_HIDDEN,
        animationSpec = SheetAnimationSpec,
      )

    PlatformBackHandler(enabled = !dismissed) { requestDismiss() }

    val stateOffset = if (anchoredState.offset.isNaN()) containerHeightPx else anchoredState.offset
    val offset = if (isIntrinsic) stateOffset else stateOffset + offsetCorrection.value
    val animatedOffsetPx = offset.roundToInt().coerceAtLeast(0)
    val intrinsicTopLimit = intrinsicTopLimitPx.roundToInt()
    val minStopHeightPx =
      (containerHeightPx - (visibleOffsets.maxOrNull() ?: containerHeightPx))
        .roundToInt()
        .coerceAtLeast(0)
    val minVisibleOffset = visibleOffsets.minOrNull() ?: containerHeightPx
    val scrimAlpha =
      if (containerHeightPx > minVisibleOffset) {
        (1f - (offset - minVisibleOffset) / (containerHeightPx - minVisibleOffset)).coerceIn(0f, 1f)
      } else {
        0f
      }

    Box(
      Modifier.fillMaxSize()
        .graphicsLayer { alpha = scrimAlpha }
        .background(AppTheme.colors.scrim)
        .clickable { requestDismiss() }
    )

    Column(
      modifier =
        Modifier.fillMaxWidth()
          .nestedScroll(nestedScrollConnection)
          .layout { measurable, constraints ->
            val maxH =
              if (isIntrinsic) {
                (constraints.maxHeight - intrinsicTopLimit).coerceAtLeast(0)
              } else {
                maxOf((constraints.maxHeight - animatedOffsetPx).coerceAtLeast(0), minStopHeightPx)
              }
            val placeable = measurable.measure(constraints.copy(maxHeight = maxH))
            val shouldUseMeasuredIntrinsicOffset =
              isIntrinsic &&
                anchoredState.settledValue == ANCHOR_VISIBLE &&
                anchoredState.targetValue == ANCHOR_VISIBLE &&
                contentHeightPx > 0f &&
                contentHeightPx != placeable.height.toFloat()
            val currentOffset =
              if (shouldUseMeasuredIntrinsicOffset) {
                maxOf(constraints.maxHeight - placeable.height, intrinsicTopLimit)
              } else {
                animatedOffsetPx
              }
            layout(placeable.width, placeable.height) { placeable.place(0, currentOffset) }
          }
          .anchoredDraggable(
            state = anchoredState,
            orientation = Orientation.Vertical,
            overscrollEffect = dragOverscrollEffect,
          )
          .thenIf(isIntrinsic) { onSizeChanged { contentHeightPx = it.height.toFloat() } }
          .clip(RoundedCornerShape(topStart = AppShapes.xl, topEnd = AppShapes.xl))
    ) {
      CompositionLocalProvider(LocalViewModelStoreOwner provides viewModelStoreOwner) {
        context(scope) { typedEntry.content() }
      }
    }
  }
}
