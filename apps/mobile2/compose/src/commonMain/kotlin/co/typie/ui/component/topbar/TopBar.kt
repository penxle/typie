package co.typie.ui.component.topbar

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.snap
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ext.statusBars
import co.typie.ext.toDp
import co.typie.ext.toPx

@Composable
fun TopBar(
  state: TopBarState,
  modifier: Modifier = Modifier,
  onTap: (() -> Unit)? = null,
) {
  val density = LocalDensity.current
  val centerSlideOffset = 16.dp.toPx(density).toInt()
  val hasScrollReveal = state.scrollOffset != null
  val centerRevealed by remember(state.scrollOffset, density) {
    derivedStateOf {
      val offset = state.scrollOffset?.invoke() ?: return@derivedStateOf true
      offset.toDp(density) > TopBarDefaults.RevealOffset
    }
  }

  val visibilityAlpha by animateFloatAsState(
    targetValue = if (state.visible) 1f else 0f,
    animationSpec = tween(TopBarDefaults.VisibilityFadeDuration),
  )
  val visibilityOffsetY by animateFloatAsState(
    targetValue = if (state.visible) 0f else -1f,
    animationSpec = tween(
      TopBarDefaults.VisibilityAnimationDuration,
      easing = EaseOutCubic,
    ),
  )

  Box(
    modifier = modifier.fillMaxWidth().graphicsLayer {
      alpha = visibilityAlpha
      translationY = visibilityOffsetY * size.height
    }.windowInsetsPadding(WindowInsets.statusBars)
      .then(if (onTap != null) Modifier.pointerInput(onTap) {
        detectTapGestures { onTap() }
      } else Modifier),
  ) {
    val topBarMode = if (state.customKey != TopBarState.NullKey) state.customKey else TopBarState.NormalModeKey

    AnimatedContent(
      targetState = topBarMode,
      transitionSpec = {
        val direction = if (targetState != TopBarState.NormalModeKey) 1 else -1
        (slideInVertically { it / 2 * direction } + fadeIn(tween(200)))
          .togetherWith(slideOutVertically { -it / 2 * direction } + fadeOut(tween(150)))
          .using(SizeTransform(clip = false) { _, _ -> snap() })
      },
    ) { mode ->
      if (mode == TopBarState.NormalModeKey) {
        Row(
          verticalAlignment = Alignment.CenterVertically,
          modifier = Modifier.fillMaxWidth().height(TopBarDefaults.Height)
            .padding(horizontal = TopBarDefaults.HorizontalPadding),
        ) {
          // Leading slot — slide + fade
          Box(
            contentAlignment = Alignment.CenterStart,
            modifier = Modifier.width(TopBarDefaults.SlotWidth).height(TopBarDefaults.Height),
          ) {
            AnimatedContent(
              targetState = state.leadingKey,
              contentAlignment = Alignment.CenterStart,
              transitionSpec = {
                (slideInHorizontally { -it / 2 } + fadeIn(tween(200))).togetherWith(slideOutHorizontally { -it / 2 } + fadeOut(
                  tween(150)
                )).using(SizeTransform(clip = false) { _, _ -> snap() })
              },
            ) { key ->
              state.leadingEntries[key]?.invoke()
            }
          }

          Spacer(Modifier.width(TopBarDefaults.SlotGap))

          // Center slot — crossfade (per route) + scroll-based reveal (per entry, independent)
          Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier.weight(1f).height(TopBarDefaults.Height),
          ) {
            AnimatedContent(
              targetState = state.centerKey,
              modifier = Modifier.fillMaxWidth(),
              contentAlignment = Alignment.Center,
              transitionSpec = {
                when {
                  initialState == TopBarState.NullKey || targetState == TopBarState.NullKey ->
                    fadeIn(tween(200)).togetherWith(fadeOut(tween(150)))
                  state.navDirection == NavDirection.Switch ->
                    fadeIn(tween(200)).togetherWith(fadeOut(tween(150)))
                  else -> {
                    val direction = if (state.navDirection == NavDirection.Push) 1 else -1
                    (slideInVertically { centerSlideOffset * direction } + fadeIn(tween(200))).togetherWith(
                      slideOutVertically { -centerSlideOffset * direction } + fadeOut(tween(150)))
                  }
                }.using(SizeTransform(clip = false) { _, _ -> snap() })
              },
            ) { key ->
              // Per-entry reveal state: updated only while this key is current, frozen on exit.
              val isCurrentKey = key == state.centerKey
              var revealed by remember { mutableStateOf(!hasScrollReveal || centerRevealed) }
              if (isCurrentKey) {
                revealed = !hasScrollReveal || centerRevealed
              }

              TopBarCenterReveal(visible = revealed) {
                state.centerEntries[key]?.invoke()
              }
            }
          }

          Spacer(Modifier.width(TopBarDefaults.SlotGap))

          // Trailing slot — slide + fade (오른쪽, leading의 반대)
          Box(
            contentAlignment = Alignment.CenterEnd,
            modifier = Modifier.width(TopBarDefaults.SlotWidth).height(TopBarDefaults.Height),
          ) {
            AnimatedContent(
              targetState = state.trailingKey,
              contentAlignment = Alignment.CenterEnd,
              transitionSpec = {
                (slideInHorizontally { it / 2 } + fadeIn(tween(200)))
                  .togetherWith(slideOutHorizontally { it / 2 } + fadeOut(tween(150)))
                  .using(SizeTransform(clip = false) { _, _ -> snap() })
              },
            ) { key ->
              state.trailingEntries[key]?.invoke()
            }
          }
        }
      } else {
        Box(
          modifier = Modifier.fillMaxWidth().height(TopBarDefaults.Height)
            .padding(horizontal = TopBarDefaults.HorizontalPadding),
          contentAlignment = Alignment.CenterStart,
        ) {
          state.customEntries[mode]?.invoke()
        }
      }
    }
  }
}

@Composable
private fun TopBarCenterReveal(
  visible: Boolean,
  content: @Composable () -> Unit,
) {
  AnimatedVisibility(
    visible = visible,
    modifier = Modifier.fillMaxWidth(),
    enter = fadeIn(tween(TopBarDefaults.RevealFadeDuration)) + slideInVertically(
      animationSpec = tween(
        TopBarDefaults.RevealAnimationDuration,
        easing = EaseOut,
      ),
      initialOffsetY = { (it * 0.4f).toInt() },
    ),
    exit = fadeOut(tween(TopBarDefaults.RevealFadeDuration)) + slideOutVertically(
      animationSpec = tween(
        TopBarDefaults.RevealAnimationDuration,
        easing = EaseOut,
      ),
      targetOffsetY = { (it * 0.4f).toInt() },
    ),
  ) {
    Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
      content()
    }
  }
}

fun ScrollState.topBarScrollOffset(): () -> Int = { value }

fun LazyListState.topBarScrollOffset(): () -> Int = {
  if (firstVisibleItemIndex > 0) Int.MAX_VALUE
  else firstVisibleItemScrollOffset
}
