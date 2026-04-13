package co.typie.ui.component.sheet

import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.AnchoredDraggableState
import androidx.compose.foundation.gestures.DraggableAnchors
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.anchoredDraggable
import androidx.compose.foundation.gestures.animateTo
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
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
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.layout
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import androidx.lifecycle.ViewModelStore
import androidx.lifecycle.ViewModelStoreOwner
import androidx.lifecycle.viewmodel.compose.LocalViewModelStoreOwner
import co.typie.ext.clickable
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.launch

private const val ANCHOR_HIDDEN = -1
private const val SHEET_CORNER_RADIUS = 22
private const val SHEET_ANIM_MS = 300

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

  val handleDismissed: () -> Unit = {
    if (!dismissed) {
      dismissed = true
      onResolve(if (resolved) pendingResult else null)
    }
  }

  BoxWithConstraints(Modifier.fillMaxSize()) {
    val density = LocalDensity.current
    val containerHeightPx = with(density) { maxHeight.toPx() }
    val isIntrinsic = entry.stops.isEmpty()
    var contentHeightPx by remember { mutableStateOf(0f) }
    val coroutineScope = rememberCoroutineScope()

    val visibleOffsets: List<Float> =
      remember(entry.stops, containerHeightPx, contentHeightPx) {
        if (isIntrinsic) {
          if (contentHeightPx > 0f) listOf(containerHeightPx - contentHeightPx) else emptyList()
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

    LaunchedEffect(anchors) { anchoredState.updateAnchors(anchors) }

    LaunchedEffect(visibleOffsets) {
      if (visibleOffsets.isEmpty()) return@LaunchedEffect

      anchoredState.animateTo(0, tween(SHEET_ANIM_MS))

      snapshotFlow { anchoredState.settledValue }
        .filter { it == ANCHOR_HIDDEN }
        .collect { handleDismissed() }
    }

    val requestDismiss: () -> Unit = {
      if (!dismissed) {
        coroutineScope.launch {
          anchoredState.animateTo(ANCHOR_HIDDEN, tween(SHEET_ANIM_MS))
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

    PlatformBackHandler(enabled = !dismissed) { requestDismiss() }

    val offset = if (anchoredState.offset.isNaN()) containerHeightPx else anchoredState.offset
    val minVisibleOffset = visibleOffsets.minOrNull() ?: containerHeightPx
    val scrimAlpha =
      if (containerHeightPx > minVisibleOffset) {
        (1f - (offset - minVisibleOffset) / (containerHeightPx - minVisibleOffset)).coerceIn(0f, 1f)
      } else {
        0f
      }

    Box(
      Modifier.fillMaxSize().alpha(scrimAlpha).background(AppTheme.colors.scrim).clickable {
        requestDismiss()
      }
    )

    Column(
      modifier =
        Modifier.fillMaxWidth()
          .layout { measurable, constraints ->
            val currentOffset = offset.roundToInt().coerceAtLeast(0)
            val maxH =
              if (isIntrinsic) {
                constraints.maxHeight
              } else {
                val maxVisibleOffset = visibleOffsets.maxOrNull() ?: containerHeightPx
                val minStopHeight =
                  (containerHeightPx - maxVisibleOffset).roundToInt().coerceAtLeast(0)
                maxOf((constraints.maxHeight - currentOffset).coerceAtLeast(0), minStopHeight)
              }
            val placeable = measurable.measure(constraints.copy(maxHeight = maxH))
            layout(placeable.width, placeable.height) { placeable.place(0, currentOffset) }
          }
          .anchoredDraggable(state = anchoredState, orientation = Orientation.Vertical)
          .then(
            if (isIntrinsic) Modifier.onSizeChanged { contentHeightPx = it.height.toFloat() }
            else Modifier
          )
          .clip(
            RoundedCornerShape(topStart = SHEET_CORNER_RADIUS.dp, topEnd = SHEET_CORNER_RADIUS.dp)
          )
          .background(AppTheme.colors.surfaceRaised)
    ) {
      SheetHandle()
      CompositionLocalProvider(LocalViewModelStoreOwner provides viewModelStoreOwner) {
        context(scope) { typedEntry.content() }
      }
    }
  }
}

private val HandleTopPadding = 8.dp
private val HandleHeight = 4.dp
private val HandleBottomPadding = 8.dp
private val HandleWidth = 36.dp

@Composable
private fun SheetHandle() {
  Box(
    modifier =
      Modifier.fillMaxWidth().height(HandleTopPadding + HandleHeight + HandleBottomPadding),
    contentAlignment = Alignment.Center,
  ) {
    Box(
      modifier =
        Modifier.size(width = HandleWidth, height = HandleHeight)
          .clip(RoundedCornerShape(HandleHeight / 2))
          .background(AppTheme.colors.borderSubtle)
    )
  }
}
