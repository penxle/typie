package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.changedToUp
import androidx.compose.ui.input.pointer.pointerInput
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.LocalHazeState
import co.typie.ui.theme.shadow
import dev.chrisbanes.haze.blur.blurEffect
import dev.chrisbanes.haze.hazeEffect
import kotlin.math.abs
import kotlin.math.roundToInt

@Composable
internal fun EditorToolbarIndicatorPill(
  pages: List<EditorToolbarPage>,
  pageProgress: Float,
  animateBackground: Boolean,
  currentPageIndex: Int,
  modifier: Modifier = Modifier,
) {
  val hazeState = LocalHazeState.current
  val surfaceColor = AppTheme.colors.surfaceDefault
  val animatedPageProgress by
    animateFloatAsState(
      targetValue = pageProgress,
      animationSpec = tween(ToolbarIndicatorBackgroundMillis),
      label = "editor-toolbar-indicator-background-progress",
    )
  val visualPageProgress = if (animateBackground) animatedPageProgress else pageProgress
  val indicatorWidth =
    ToolbarIndicatorPadding * 2 +
      ToolbarIndicatorItemSize * pages.size +
      ToolbarIndicatorItemGap * (pages.size - 1)
  val animatedIndicatorWidth by
    animateDpAsState(
      targetValue = indicatorWidth,
      animationSpec = tween(ToolbarIndicatorWidthMillis),
      label = "editor-toolbar-indicator-width",
    )
  val indicatorItems = pages.map { page ->
    ToolbarIndicatorIconItem(
      key = page.key,
      icon = page.icon,
      contentDescription = page.contentDescription,
    )
  }

  Box(
    modifier =
      modifier
        .width(animatedIndicatorWidth)
        .height(ToolbarIndicatorHeight)
        .shadow(AppTheme.shadows.sm, ToolbarIndicatorShape)
        .clip(ToolbarIndicatorShape)
        .hazeEffect(hazeState) {
          blurEffect {
            backgroundColor = surfaceColor
            blurRadius = ToolbarBackdropBlurRadius
          }
        }
        .border(ToolbarBorderWidth, AppTheme.colors.borderEmphasis, ToolbarIndicatorShape)
  ) {
    EditorToolbarSurfaceBackground(shape = ToolbarIndicatorShape)

    Box(
      modifier =
        Modifier.offset(
            x =
              ToolbarIndicatorPadding +
                (ToolbarIndicatorItemSize + ToolbarIndicatorItemGap) * visualPageProgress,
            y = ToolbarIndicatorPadding,
          )
          .size(ToolbarIndicatorItemSize)
          .background(AppTheme.colors.surfaceInset, ToolbarIndicatorShape)
    )

    AnimatedContent(
      targetState = indicatorItems,
      transitionSpec = {
        (fadeIn(tween(ToolbarIndicatorIconsMillis)) +
            scaleIn(tween(ToolbarIndicatorIconsMillis), initialScale = 0.92f))
          .togetherWith(
            fadeOut(tween(ToolbarIndicatorIconsMillis)) +
              scaleOut(tween(ToolbarIndicatorIconsMillis), targetScale = 0.92f)
          )
      },
      modifier = Modifier.fillMaxSize(),
      label = "editor-toolbar-indicator-icons",
    ) { targetItems ->
      Row(
        modifier = Modifier.fillMaxSize().padding(ToolbarIndicatorPadding),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(ToolbarIndicatorItemGap),
      ) {
        targetItems.forEachIndexed { index, item ->
          Box(
            modifier = Modifier.size(ToolbarIndicatorItemSize).focusProperties { canFocus = false },
            contentAlignment = Alignment.Center,
          ) {
            Icon(
              icon = item.icon,
              contentDescription = item.contentDescription,
              modifier = Modifier.size(ToolbarIndicatorIconSize),
              tint =
                if (index == currentPageIndex) AppTheme.colors.textDefault
                else AppTheme.colors.textHint,
            )
          }
        }
      }
    }
  }
}

private data class ToolbarIndicatorIconItem(
  val key: EditorToolbarPageKey,
  val icon: IconData,
  val contentDescription: String,
)

@Composable
internal fun Modifier.toolbarIndicatorGestures(
  pageCount: Int,
  currentPageIndex: Int,
  onIndicatorProgress: (Float) -> Unit,
  onIndicatorDraggingChange: (Boolean) -> Unit,
  onPageSelected: (Int) -> Unit,
  onInteractionActiveChange: (Boolean) -> Unit,
): Modifier {
  val latestCurrentPageIndex = rememberUpdatedState(currentPageIndex)
  val latestOnIndicatorProgress = rememberUpdatedState(onIndicatorProgress)
  val latestOnIndicatorDraggingChange = rememberUpdatedState(onIndicatorDraggingChange)
  val latestOnPageSelected = rememberUpdatedState(onPageSelected)
  val latestOnInteractionActiveChange = rememberUpdatedState(onInteractionActiveChange)

  return pointerInput(pageCount) {
    if (pageCount <= 1) return@pointerInput

    fun progressForX(x: Float): Float {
      val paddingPx = ToolbarIndicatorPadding.toPx()
      val itemPx = ToolbarIndicatorItemSize.toPx()
      val gapPx = ToolbarIndicatorItemGap.toPx()
      val firstCenter = paddingPx + itemPx / 2f
      val lastCenter = firstCenter + (itemPx + gapPx) * (pageCount - 1)
      return ((x.coerceIn(firstCenter, lastCenter) - firstCenter) / (lastCenter - firstCenter) *
          (pageCount - 1))
        .coerceIn(0f, (pageCount - 1).toFloat())
    }

    fun pageForX(x: Float): Int = progressForX(x).roundToInt().coerceIn(0, pageCount - 1)

    awaitEachGesture {
      val down = awaitFirstDown(requireUnconsumed = false)
      var lastPosition = down.position
      var totalDelta = Offset.Zero
      val downProgress = progressForX(down.position.x)
      val downPage = pageForX(down.position.x)
      var dispatchedPage = latestCurrentPageIndex.value
      var dragging = false
      var followingPointer = false

      fun followPointer(x: Float) {
        latestOnIndicatorProgress.value(progressForX(x))
        val page = pageForX(x)
        if (page != dispatchedPage) {
          dispatchedPage = page
          latestOnPageSelected.value(page)
        }
      }

      latestOnInteractionActiveChange.value(true)
      latestOnIndicatorDraggingChange.value(false)
      latestOnIndicatorProgress.value(downProgress)
      if (downPage != dispatchedPage) {
        dispatchedPage = downPage
        latestOnPageSelected.value(downPage)
      }

      try {
        while (true) {
          val event = awaitPointerEvent()
          val change = event.changes.firstOrNull { it.id == down.id } ?: break

          if (change.changedToUp()) {
            latestOnIndicatorProgress.value(progressForX(change.position.x))
            val page = pageForX(change.position.x)
            if (page != dispatchedPage) {
              latestOnPageSelected.value(page)
            }
            break
          }

          if (change.isConsumed) break

          val delta = change.position - lastPosition
          totalDelta += delta
          lastPosition = change.position

          if (
            !followingPointer && delta.getDistance() > ToolbarIndicatorFollowMovementThresholdPx
          ) {
            followingPointer = true
            latestOnIndicatorDraggingChange.value(true)
          }

          if (followingPointer) {
            followPointer(change.position.x)
          }

          if (!dragging) {
            if (abs(totalDelta.x) > viewConfiguration.touchSlop) {
              dragging = true
              change.consume()
            } else if (abs(totalDelta.y) > viewConfiguration.touchSlop) {
              break
            }
          }

          if (dragging) {
            change.consume()
          }
        }
      } finally {
        latestOnIndicatorDraggingChange.value(false)
        latestOnInteractionActiveChange.value(false)
      }
    }
  }
}
