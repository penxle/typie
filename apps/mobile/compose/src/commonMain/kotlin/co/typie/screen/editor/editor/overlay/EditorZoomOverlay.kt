package co.typie.screen.editor.editor.overlay

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsHoveredAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.sizeIn
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.MotionDurationScale
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.onPointerEvent
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorZoomLandmark
import co.typie.editor.LocalEditorZoomController
import co.typie.ext.LocalInteractionSource
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.layout.viewportDirectControl
import co.typie.screen.editor.editor.toolbar.emitPressInteractions
import co.typie.screen.editor.editor.toolbar.preserveEditorFocusOnToolbarInteraction
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.input.hasNonTouchPointer
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlin.math.roundToInt
import kotlinx.coroutines.delay

private const val ZOOM_LANDMARK_VISIBLE_MS = 1000L
private const val ZOOM_OVERLAY_FADE_MS = 180
private const val ZOOM_VALUE_TRANSITION_MS = 160
private val ZoomPointerValueWidth = 68.dp
private val ZoomOverlayShape = AppShapes.rounded(8.dp)
private val ZoomButtonShape = AppShapes.rounded(8.dp)
private val ZoomValueEasing = CubicBezierEasing(0.2f, 0f, 0f, 1f)

private data class ZoomValuePresentation(
  val text: String,
  val icon: IconData,
  val isLandmark: Boolean,
)

@OptIn(ExperimentalComposeUiApi::class)
@Composable
internal fun EditorZoomOverlay(
  state: EditorZoomIndicatorState,
  nonTouchPointerActive: Boolean,
  onZoomOut: () -> Boolean,
  onZoomIn: () -> Boolean,
  onToggleZoom: () -> EditorZoomLandmark?,
  onRequestEditorFocus: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val zoomController = LocalEditorZoomController.current
  val enabled = zoomController.isZoomEnabled
  val displayZoom = zoomController.displayZoom
  val indicatorZoom = zoomController.indicatorZoom
  val landmark = zoomController.resolveLandmark()
  val zoomOutAvailable = zoomController.resolveZoomOutTarget() != null
  val zoomInAvailable = zoomController.resolveZoomInTarget() != null
  val toggleTarget = zoomController.resolveIndicatorToggleTarget()
  val toggleTargetLandmark = toggleTarget?.let(zoomController::resolveLandmark)
  val toggleDescription = toggleDescription(toggleTargetLandmark)

  LaunchedEffect(enabled, displayZoom, indicatorZoom, landmark) {
    state.updateZoom(
      enabled = enabled,
      displayZoom = displayZoom,
      indicatorZoom = indicatorZoom,
      landmark = landmark,
    )
  }
  LaunchedEffect(state.visibilityRequest) {
    val request = state.visibilityRequest
    if (request <= 0) return@LaunchedEffect
    delay(state.autoHideDelayMillis)
    state.expireVisibility(request)
  }
  LaunchedEffect(state.landmarkRequest) {
    val request = state.landmarkRequest
    if (request <= 0 || state.landmarkHeld) return@LaunchedEffect
    delay(ZOOM_LANDMARK_VISIBLE_MS)
    state.expireLandmark(request)
  }
  DisposableEffect(state) { onDispose(state::reset) }

  if (!enabled) return

  val alpha by
    animateFloatAsState(
      targetValue = if (state.visible) 1f else 0f,
      animationSpec = tween(durationMillis = ZOOM_OVERLAY_FADE_MS),
      label = "editor-zoom-overlay-alpha",
    )
  if (!state.visible && alpha == 0f && !nonTouchPointerActive) return

  val zoomPercent = (indicatorZoom * 100f).roundToInt()
  val displayedLandmark = state.displayedLandmark(landmark)
  val valuePresentation =
    ZoomValuePresentation(
      text = state.displayText(landmark = landmark, zoomPercent = zoomPercent),
      icon = displayedLandmark?.indicatorIcon ?: Lucide.Search,
      isLandmark = displayedLandmark != null,
    )

  Box(
    modifier =
      modifier
        .graphicsLayer { this.alpha = alpha }
        .onFocusChanged { state.onFocusChanged(it.hasFocus) }
  ) {
    Row(
      modifier =
        Modifier.clip(ZoomOverlayShape)
          .border(1.dp, AppTheme.colors.borderEmphasis, ZoomOverlayShape)
          .background(
            lerp(AppTheme.colors.surfaceDefault, AppTheme.colors.surfaceInset, 0.15f)
              .copy(alpha = 0.95f),
            ZoomOverlayShape,
          )
          .onPointerEvent(PointerEventType.Enter, PointerEventPass.Initial) { event ->
            if (event.hasNonTouchPointer()) {
              state.onIndicatorPointerEnter()
            }
          }
          .onPointerEvent(PointerEventType.Exit, PointerEventPass.Initial) {
            state.onIndicatorPointerExit()
          }
          .then(
            if (state.visible) {
              Modifier.viewportDirectControl().preserveEditorFocusOnToolbarInteraction()
            } else {
              Modifier
            }
          )
          .padding(4.dp),
      verticalAlignment = Alignment.CenterVertically,
    ) {
      if (nonTouchPointerActive) {
        ZoomIconButton(
          icon = Lucide.Minus,
          contentDescription = if (zoomOutAvailable) "페이지 축소" else "최소 배율입니다",
          enabled = state.visible,
          available = zoomOutAvailable,
          onClick = {
            try {
              applyZoomStep(onZoomOut, state, EditorZoomLandmark.Minimum)
            } finally {
              onRequestEditorFocus()
            }
          },
        )
      }

      ZoomValueButton(
        presentation = valuePresentation,
        snapFeedbackRequest = state.snapFeedbackRequest,
        snapFeedbackLandmark = state.snapFeedbackLandmark,
        contentDescription = toggleDescription,
        fixedWidth = nonTouchPointerActive,
        enabled = state.visible && toggleTarget != null,
        onPointerEnter = state::onValuePointerEnter,
        onPointerExit = state::onValuePointerExit,
        onFocusChanged = state::onValueFocusChanged,
        onClick = {
          try {
            onToggleZoom()
          } finally {
            onRequestEditorFocus()
          }
        },
      )

      if (nonTouchPointerActive) {
        ZoomIconButton(
          icon = Lucide.Plus,
          contentDescription = if (zoomInAvailable) "페이지 확대" else "최대 배율입니다",
          enabled = state.visible,
          available = zoomInAvailable,
          onClick = {
            try {
              applyZoomStep(onZoomIn, state, EditorZoomLandmark.Maximum)
            } finally {
              onRequestEditorFocus()
            }
          },
        )
      }
    }
  }
}

@Composable
private fun ZoomIconButton(
  icon: IconData,
  contentDescription: String,
  enabled: Boolean,
  available: Boolean,
  onClick: () -> Unit,
) {
  val interactionSource = remember { MutableInteractionSource() }
  val hovered by interactionSource.collectIsHoveredAsState()
  CompositionLocalProvider(LocalInteractionSource provides interactionSource) {
    Box(
      modifier =
        Modifier.size(28.dp)
          .pressScale()
          .focusProperties { canFocus = false }
          .clip(ZoomButtonShape)
          .background(
            if (hovered && enabled) AppTheme.colors.borderDefault else Color.Transparent,
            ZoomButtonShape,
          )
          .clickable(
            enabled = enabled,
            interactionSource = interactionSource,
            indication = null,
            role = Role.Button,
            onClick = onClick,
          )
          .acceptDirectControlClick(
            enabled = enabled,
            interactionSource = interactionSource,
            onClick = onClick,
          )
          .semantics { this.contentDescription = contentDescription },
      contentAlignment = Alignment.Center,
    ) {
      Icon(
        icon = icon,
        contentDescription = null,
        modifier = Modifier.size(14.dp),
        tint = if (enabled && available) AppTheme.colors.textMuted else AppTheme.colors.textHint,
      )
    }
  }
}

@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun ZoomValueButton(
  presentation: ZoomValuePresentation,
  snapFeedbackRequest: Int,
  snapFeedbackLandmark: EditorZoomLandmark?,
  contentDescription: String,
  fixedWidth: Boolean,
  enabled: Boolean,
  onPointerEnter: () -> Unit,
  onPointerExit: () -> Unit,
  onFocusChanged: (Boolean) -> Unit,
  onClick: () -> Unit,
) {
  val interactionSource = remember { MutableInteractionSource() }
  val hovered by interactionSource.collectIsHoveredAsState()
  CompositionLocalProvider(LocalInteractionSource provides interactionSource) {
    Box(
      modifier =
        Modifier.sizeIn(minWidth = 36.dp, minHeight = 28.dp)
          .then(if (fixedWidth) Modifier.width(ZoomPointerValueWidth) else Modifier)
          .pressScale()
          .focusProperties { canFocus = false }
          .clip(ZoomButtonShape)
          .border(1.dp, Color.Transparent, ZoomButtonShape)
          .background(
            if (hovered && enabled) AppTheme.colors.borderDefault else Color.Transparent,
            ZoomButtonShape,
          )
          .onPointerEvent(PointerEventType.Enter, PointerEventPass.Initial) { event ->
            if (event.hasNonTouchPointer()) onPointerEnter()
          }
          .onPointerEvent(PointerEventType.Exit, PointerEventPass.Initial) { onPointerExit() }
          .onFocusChanged { onFocusChanged(it.isFocused) }
          .clickable(
            enabled = enabled,
            interactionSource = interactionSource,
            indication = null,
            role = Role.Button,
            onClick = onClick,
          )
          .acceptDirectControlClick(
            enabled = enabled,
            interactionSource = interactionSource,
            onClick = onClick,
          )
          .semantics { this.contentDescription = contentDescription },
      contentAlignment = Alignment.Center,
    ) {
      ZoomSnapBorderFlash(request = snapFeedbackRequest, landmark = snapFeedbackLandmark)
      AnimatedContent(
        modifier = Modifier.padding(horizontal = 6.dp),
        targetState = presentation,
        contentKey = ZoomValuePresentation::isLandmark,
        transitionSpec = {
          val contentTransition =
            if (initialState.isLandmark != targetState.isLandmark) {
              fadeIn(tween(ZOOM_VALUE_TRANSITION_MS)) togetherWith
                fadeOut(tween(ZOOM_VALUE_TRANSITION_MS))
            } else {
              EnterTransition.None togetherWith ExitTransition.None
            }
          if (fixedWidth) {
            contentTransition
          } else {
            contentTransition.using(
              SizeTransform(clip = false) { _, _ ->
                tween(durationMillis = ZOOM_VALUE_TRANSITION_MS, easing = ZoomValueEasing)
              }
            )
          }
        },
        contentAlignment = Alignment.Center,
        label = "editor-zoom-value",
      ) { target ->
        Row(
          horizontalArrangement = Arrangement.spacedBy(4.dp),
          verticalAlignment = Alignment.CenterVertically,
        ) {
          Icon(
            icon = target.icon,
            contentDescription = null,
            modifier = Modifier.size(14.dp),
            tint = AppTheme.colors.textMuted,
          )
          Text(
            text = target.text,
            style = AppTheme.typography.caption.copy(fontWeight = FontWeight.W500),
            color = AppTheme.colors.textDefault,
            textAlign = TextAlign.Center,
            softWrap = false,
            maxLines = 1,
          )
        }
      }
    }
  }
}

private fun Modifier.acceptDirectControlClick(
  enabled: Boolean,
  interactionSource: MutableInteractionSource,
  onClick: () -> Unit,
): Modifier =
  if (enabled) {
    emitPressInteractions(interactionSource).pointerInput(onClick) {
      detectTapAfterConsumedDown(onClick)
    }
  } else {
    this
  }

@Composable
private fun BoxScope.ZoomSnapBorderFlash(request: Int, landmark: EditorZoomLandmark?) {
  if (request <= 0 || landmark == null) return

  val coroutineScope = rememberCoroutineScope()
  val reducedMotion =
    coroutineScope.coroutineContext[MotionDurationScale]
      ?.scaleFactor
      ?.takeIf { it.isFinite() }
      ?.let { it <= 0f } ?: false

  if (reducedMotion) {
    var visible by remember(request) { mutableStateOf(true) }
    LaunchedEffect(request) {
      delay(280L)
      visible = false
    }
    if (visible) {
      Box(
        modifier =
          Modifier.matchParentSize().border(1.dp, AppTheme.colors.borderDefault, ZoomButtonShape)
      )
    }
    return
  }

  val alpha = remember(request) { Animatable(0f) }
  LaunchedEffect(request) {
    alpha.animateTo(1f, tween(durationMillis = 70))
    alpha.animateTo(0f, tween(durationMillis = 310))
  }
  Box(
    modifier =
      Modifier.matchParentSize()
        .graphicsLayer { this.alpha = alpha.value }
        .border(2.dp, AppTheme.colors.borderDefault, ZoomButtonShape)
  )
}

private fun applyZoomStep(
  step: () -> Boolean,
  state: EditorZoomIndicatorState,
  boundary: EditorZoomLandmark,
) {
  if (!step()) state.onBoundaryAttempt(boundary)
}

private fun toggleDescription(landmark: EditorZoomLandmark?): String =
  when (landmark) {
    EditorZoomLandmark.FitWidth -> "화면에 맞추기"
    EditorZoomLandmark.Unit -> "원본 크기로 돌아가기  ⌘/Ctrl 0"
    EditorZoomLandmark.Minimum -> "최소 배율로 축소"
    EditorZoomLandmark.Maximum -> "최대 배율로 확대"
    null -> "원본 크기가 화면에 맞춰져 있어요"
  }

private val EditorZoomLandmark.indicatorIcon: IconData
  get() =
    when (this) {
      EditorZoomLandmark.Minimum -> Lucide.ZoomOut
      EditorZoomLandmark.FitWidth -> Lucide.GalleryHorizontal
      EditorZoomLandmark.Unit -> Lucide.FileCheckCorner
      EditorZoomLandmark.Maximum -> Lucide.ZoomIn
    }
