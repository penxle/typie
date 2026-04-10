package co.typie.ui.component.sheet

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.dropShadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ui.theme.AppTheme

data class PersistentSheetSpec(
  val sizePolicy: SheetSizePolicy = SheetSizePolicy.Intrinsic(),
  val chrome: SheetChrome = SheetChrome.Default,
  val haptics: SheetHapticPolicy = SheetHapticPolicy(onPresent = false),
)

class PersistentSheetState internal constructor(
  internal val controller: SheetControllerState<Unit>,
  internal val spec: PersistentSheetSpec,
) {
  var visible by mutableStateOf(true)
    private set

  fun show() {
    visible = true
  }

  fun hide() {
    visible = false
  }
}

@Composable
fun rememberPersistentSheetState(
  spec: PersistentSheetSpec = PersistentSheetSpec(),
): PersistentSheetState {
  return remember(spec) {
    PersistentSheetState(
      controller = SheetControllerState(
        mode = SheetMode.Persistent,
        dismissPolicy = SheetDismissPolicy(),
      ),
      spec = spec,
    )
  }
}

@Composable
fun PersistentSheet(
  state: PersistentSheetState,
  modifier: Modifier = Modifier,
  content: @Composable SheetScope<Unit>.() -> Unit,
) {
  if (!state.visible) return

  val density = LocalDensity.current
  var measuredSheetHeightPx by remember { mutableFloatStateOf(0f) }
  var sheetHeightPx by remember { mutableFloatStateOf(0f) }
  val colors = AppTheme.colors
  val sheetScope = remember(state) {
    object : SheetScope<Unit> {
      override val controller: SheetController<Unit> = state.controller

      override fun complete(result: Unit) = Unit

      override fun dismiss(reason: SheetDismissReason) = Unit
    }
  }

  BoxWithConstraints(
    modifier = modifier.fillMaxWidth(),
  ) {
    val resolvedDetents = remember(
      state.spec.sizePolicy,
      maxHeight,
      measuredSheetHeightPx,
    ) {
      resolveDetentsForSheetMeasurement(
        policy = state.spec.sizePolicy,
        viewportHeight = maxHeight,
        measuredSheetHeightPx = measuredSheetHeightPx,
        density = density,
      )
    }
    val requiresContentMeasurement = remember(state.spec.sizePolicy) {
      state.spec.sizePolicy.requiresContentMeasurement()
    }
    val initialDetentId = state.spec.sizePolicy.initialDetentId()
    val initialDetent = resolvedDetents.firstOrNull { it.id == initialDetentId } ?: resolvedDetents.firstOrNull()

    LaunchedEffect(resolvedDetents) {
      if (resolvedDetents.isEmpty()) return@LaunchedEffect
      state.controller.updateResolvedDetents(
        detents = resolvedDetents,
        initialDetentId = initialDetent?.id ?: initialDetentId,
        stackDepth = 0,
        isTopOfStack = true,
      )
      val targetHeightPx = initialDetent?.let { with(density) { it.height.toPx() } } ?: return@LaunchedEffect
      if (sheetHeightPx == 0f) {
        sheetHeightPx = targetHeightPx
      } else {
        val animatable = Animatable(sheetHeightPx)
        animatable.animateTo(
          targetHeightPx,
          animationSpec = tween(
            durationMillis = SheetDefaults.HeightAnimationDuration,
            easing = SheetDefaults.EnterEasing,
          ),
        ) {
          sheetHeightPx = value
        }
      }
      state.controller.snapToCurrentTarget()
      state.controller.updateVisibleFraction(1f)
    }

    Column(
      modifier = Modifier
        .fillMaxWidth()
        .heightIn(max = maxHeight)
        .then(
          if (sheetHeightPx > 0f) Modifier.height(with(density) { sheetHeightPx.toDp() }) else Modifier,
        )
        .onSizeChanged { size ->
          if (requiresContentMeasurement) {
            val nextHeight = size.height.toFloat()
            if (nextHeight > measuredSheetHeightPx) {
              measuredSheetHeightPx = nextHeight
            }
          }
        }
        .dropShadow(
          RoundedCornerShape(
            topStart = state.spec.chrome.topCornerRadius,
            topEnd = state.spec.chrome.topCornerRadius,
          ),
        ) {
          color = colors.shadowAmbient
          radius = 8f
        }
        .dropShadow(
          RoundedCornerShape(
            topStart = state.spec.chrome.topCornerRadius,
            topEnd = state.spec.chrome.topCornerRadius,
          ),
        ) {
          color = colors.shadow
          offset = Offset(0f, -4f)
          radius = 12f
        }
        .clip(
          RoundedCornerShape(
            topStart = state.spec.chrome.topCornerRadius,
            topEnd = state.spec.chrome.topCornerRadius,
          ),
        )
        .background(colors.surfaceRaised),
    ) {
      when (val handle = state.spec.chrome.handle) {
        SheetHandleStyle.Hidden -> Unit
        is SheetHandleStyle.Visible -> {
          Spacer(modifier = Modifier.height(handle.topPadding))
          androidx.compose.foundation.layout.Box(
            modifier = Modifier
              .size(width = handle.width, height = handle.height)
              .clip(RoundedCornerShape(handle.height / 2))
              .background(colors.borderSubtle),
          )
          Spacer(modifier = Modifier.height(handle.bottomPadding))
        }
      }

      Column(
        modifier = Modifier.fillMaxWidth(),
      ) {
        content(sheetScope)
      }
    }
  }
}
