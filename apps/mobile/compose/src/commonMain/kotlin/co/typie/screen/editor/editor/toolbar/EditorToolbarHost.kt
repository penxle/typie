package co.typie.screen.editor.editor.toolbar

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.FlingBehavior
import androidx.compose.foundation.gestures.Orientation
import androidx.compose.foundation.gestures.ScrollScope
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.rememberScrollableState
import androidx.compose.foundation.gestures.scrollable
import androidx.compose.foundation.gestures.waitForUpOrCancellation
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import co.typie.ext.InteractionScope
import co.typie.ext.LocalInteractionSource
import co.typie.ext.ime
import co.typie.ext.pressScale
import co.typie.icons.Lucide
import co.typie.ui.component.ResponsiveContainerDefaults
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

private val ToolbarHorizontalPadding = 16.dp
private val ToolbarBottomPadding = 12.dp
private val ToolbarHeight = 44.dp
private val ToolbarButtonSize = 30.dp
private val ToolbarPageHorizontalPadding = 8.dp
private val ToolbarPillVerticalPadding = (ToolbarHeight - ToolbarButtonSize) / 2
private val ToolbarIconSize = 20.dp
private val ToolbarFabGap = 8.dp
private val ToolbarPagePeek = 32.dp
private val ToolbarBorderWidth = 1.dp
private val ToolbarCapsuleShape = AppShapes.rounded(AppShapes.full)
private val ToolbarButtonShape = AppShapes.circle
private val ToolbarFabShape = AppShapes.circle
private const val ToolbarPrimaryPage = 0
private const val ToolbarPanelPage = 1
private const val ToolbarCapsulePressedScale = 1.015f
private const val ToolbarControlPressedScale = 1.05f
private const val ToolbarPressAnimationMs = 100
private const val ToolbarIconCrossfadeMs = 120
private const val ToolbarSwipeVelocityThreshold = 600f

private data class EditorToolbarItem(val icon: IconData, val contentDescription: String)

private val ToolbarPrimaryItems =
  listOf(
    EditorToolbarItem(icon = Lucide.Plus, contentDescription = "삽입"),
    EditorToolbarItem(icon = Lucide.Type, contentDescription = "텍스트"),
    EditorToolbarItem(icon = Lucide.Undo, contentDescription = "실행 취소"),
    EditorToolbarItem(icon = Lucide.Redo, contentDescription = "다시 실행"),
    null,
    null,
    null,
  )

private val ToolbarPanelItems =
  listOf(
    EditorToolbarItem(icon = Lucide.Search, contentDescription = "찾기"),
    EditorToolbarItem(icon = Lucide.StickyNote, contentDescription = "노트"),
    EditorToolbarItem(icon = Lucide.MessageSquareText, contentDescription = "코멘트"),
    EditorToolbarItem(icon = Lucide.SpellCheck, contentDescription = "맞춤법 검사"),
    EditorToolbarItem(icon = Lucide.Lightbulb, contentDescription = "AI 피드백"),
    EditorToolbarItem(icon = Lucide.History, contentDescription = "타임라인"),
    EditorToolbarItem(icon = Lucide.Settings, contentDescription = "본문 설정"),
  )

@OptIn(ExperimentalComposeUiApi::class)
@Composable
internal fun EditorToolbarHost(
  editorFocused: Boolean,
  visible: Boolean,
  safeBottomInset: Dp,
  onEditorFocusRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val imeBottom = WindowInsets.ime.asPaddingValues().calculateBottomPadding()
  val keyboardVisible = imeBottom > 0.dp
  val bottomInset = maxOf(imeBottom, safeBottomInset)
  val focusManager = LocalFocusManager.current
  val keyboardController = LocalSoftwareKeyboardController.current

  AnimatedVisibility(
    visible = visible,
    enter = fadeIn(),
    exit = fadeOut(),
    modifier = modifier.fillMaxWidth(),
  ) {
    Box(
      modifier =
        Modifier.fillMaxWidth()
          .offset { IntOffset(x = 0, y = -bottomInset.roundToPx()) }
          .padding(
            start = ToolbarHorizontalPadding,
            end = ToolbarHorizontalPadding,
            bottom = ToolbarBottomPadding,
          ),
      contentAlignment = Alignment.BottomCenter,
    ) {
      Row(
        modifier = Modifier.widthIn(max = ResponsiveContainerDefaults.MaxWidth).fillMaxWidth(),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
      ) {
        EditorToolbarCapsule(
          editorFocused = editorFocused,
          onEditorFocusRequest = onEditorFocusRequest,
          modifier = Modifier.weight(1f),
        )

        Spacer(Modifier.width(ToolbarFabGap))

        EditorToolbarFab(
          icon = if (keyboardVisible) Lucide.KeyboardOff else Lucide.TextCursor,
          contentDescription =
            when {
              !keyboardVisible -> "에디터 포커스"
              editorFocused -> "에디터 포커스 해제"
              else -> "키보드 닫기"
            },
          onClick = {
            when {
              !keyboardVisible -> onEditorFocusRequest()
              editorFocused -> focusManager.clearFocus()
              else -> keyboardController?.hide()
            }
          },
        )
      }
    }
  }
}

@Composable
private fun EditorToolbarCapsule(
  editorFocused: Boolean,
  onEditorFocusRequest: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val scope = rememberCoroutineScope()
  val density = LocalDensity.current
  val carouselOffset = remember { Animatable(0f) }
  var pendingFocusTargetPage by remember { mutableStateOf<Int?>(null) }

  BoxWithConstraints(modifier = modifier.height(ToolbarHeight)) {
    val pageDistance =
      with(density) { (maxWidth - ToolbarPagePeek).roundToPx().coerceAtLeast(0) }.toFloat()
    val scrollableState = rememberScrollableState { delta ->
      val currentOffset = carouselOffset.value
      val nextOffset = (currentOffset + delta).coerceIn(-pageDistance, 0f)
      val consumed = nextOffset - currentOffset

      if (consumed != 0f) {
        scope.launch {
          carouselOffset.stop()
          carouselOffset.snapTo(nextOffset)
        }
      }
      consumed
    }

    LaunchedEffect(editorFocused) {
      pendingFocusTargetPage = if (editorFocused) ToolbarPrimaryPage else ToolbarPanelPage
    }

    LaunchedEffect(pendingFocusTargetPage, pageDistance, scrollableState.isScrollInProgress) {
      val targetPage = pendingFocusTargetPage ?: return@LaunchedEffect
      if (scrollableState.isScrollInProgress) {
        return@LaunchedEffect
      }
      val targetOffset = targetPage.resolveToolbarPageOffset(pageDistance)
      carouselOffset.animateTo(targetOffset)
      pendingFocusTargetPage = null
    }

    suspend fun settleCarousel(velocity: Float = 0f) {
      val snapPage =
        resolveToolbarSnapPage(
          offset = carouselOffset.value,
          pageDistance = pageDistance,
          velocity = velocity,
        )
      if (snapPage == ToolbarPrimaryPage && !editorFocused) {
        onEditorFocusRequest()
      }
      carouselOffset.animateTo(snapPage.resolveToolbarPageOffset(pageDistance))
    }

    InteractionScope {
      val toolbarInteractionSource =
        LocalInteractionSource.current ?: remember { MutableInteractionSource() }
      val flingBehavior =
        remember(carouselOffset, pageDistance, editorFocused, onEditorFocusRequest) {
          object : FlingBehavior {
            override suspend fun ScrollScope.performFling(initialVelocity: Float): Float {
              settleCarousel(initialVelocity)
              return 0f
            }
          }
        }

      Box(
        modifier =
          Modifier.fillMaxSize()
            .pressScale(ToolbarCapsulePressedScale)
            .preserveEditorFocusOnToolbarInteraction()
            .emitPressInteractions(toolbarInteractionSource)
            .scrollable(
              state = scrollableState,
              orientation = Orientation.Horizontal,
              enabled = pageDistance > 0f,
              flingBehavior = flingBehavior,
              interactionSource = toolbarInteractionSource,
            )
            .shadow(AppTheme.shadows.sm, ToolbarCapsuleShape)
            .clip(ToolbarCapsuleShape)
            .background(AppTheme.colors.surfaceDefault, ToolbarCapsuleShape)
            .border(ToolbarBorderWidth, AppTheme.colors.borderEmphasis, ToolbarCapsuleShape)
      ) {
        Box(
          modifier =
            Modifier.fillMaxSize().offset {
              IntOffset(x = carouselOffset.value.roundToInt(), y = 0)
            }
        ) {
          EditorToolbarPrimaryPage(modifier = Modifier.fillMaxSize())
          EditorToolbarPanelPage(
            modifier =
              Modifier.fillMaxSize().offset { IntOffset(x = pageDistance.roundToInt(), y = 0) }
          )
        }
      }
    }
  }
}

private fun Int.resolveToolbarPageOffset(pageDistance: Float): Float =
  when (this) {
    ToolbarPrimaryPage -> 0f
    ToolbarPanelPage -> -pageDistance
    else -> 0f
  }

private fun resolveToolbarSnapPage(offset: Float, pageDistance: Float, velocity: Float): Int =
  when {
    velocity <= -ToolbarSwipeVelocityThreshold -> ToolbarPanelPage
    velocity >= ToolbarSwipeVelocityThreshold -> ToolbarPrimaryPage
    offset <= -pageDistance / 2f -> ToolbarPanelPage
    else -> ToolbarPrimaryPage
  }

@Composable
private fun EditorToolbarPrimaryPage(modifier: Modifier = Modifier) {
  EditorToolbarItemRow(items = ToolbarPrimaryItems, modifier = modifier)
}

@Composable
private fun EditorToolbarPanelPage(modifier: Modifier = Modifier) {
  EditorToolbarItemRow(items = ToolbarPanelItems, modifier = modifier)
}

@Composable
private fun EditorToolbarItemRow(items: List<EditorToolbarItem?>, modifier: Modifier = Modifier) {
  Row(
    modifier =
      modifier
        .fillMaxSize()
        .padding(horizontal = ToolbarPageHorizontalPadding, vertical = ToolbarPillVerticalPadding),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    items.forEach { item ->
      Box(modifier = Modifier.weight(1f).fillMaxSize(), contentAlignment = Alignment.Center) {
        if (item != null) {
          EditorToolbarButton(
            icon = item.icon,
            contentDescription = item.contentDescription,
            onClick = {},
          )
        }
      }
    }
  }
}

private fun Modifier.emitPressInteractions(interactionSource: MutableInteractionSource): Modifier =
  pointerInput(interactionSource) {
    awaitEachGesture {
      val down = awaitFirstDown(requireUnconsumed = false)
      val press = PressInteraction.Press(down.position)
      interactionSource.tryEmit(press)

      val up = waitForUpOrCancellation()
      val release =
        if (up == null) {
          PressInteraction.Cancel(press)
        } else {
          PressInteraction.Release(press)
        }
      interactionSource.tryEmit(release)
    }
  }

private fun Modifier.preserveEditorFocusOnToolbarInteraction(): Modifier =
  pointerInput(Unit) {
    awaitPointerEventScope {
      while (true) {
        val event = awaitPointerEvent(PointerEventPass.Initial)
        event.changes.forEach { change ->
          if (change.pressed && !change.previousPressed) {
            change.consume()
          }
        }
      }
    }
  }

@Composable
private fun EditorToolbarButton(
  icon: IconData,
  contentDescription: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
) {
  EditorToolbarIconButton(
    icon = icon,
    contentDescription = contentDescription,
    onClick = onClick,
    shape = ToolbarButtonShape,
    modifier = modifier.size(ToolbarButtonSize),
  )
}

@Composable
private fun EditorToolbarFab(
  icon: IconData,
  contentDescription: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
) {
  EditorToolbarIconButton(
    icon = icon,
    contentDescription = contentDescription,
    onClick = onClick,
    shape = ToolbarFabShape,
    surface = true,
    crossfadeIcon = true,
    modifier = modifier.size(ToolbarHeight),
  )
}

@Composable
private fun EditorToolbarIconButton(
  icon: IconData,
  contentDescription: String,
  onClick: () -> Unit,
  shape: Shape,
  modifier: Modifier = Modifier,
  surface: Boolean = false,
  crossfadeIcon: Boolean = false,
) {
  val interactionSource = remember { MutableInteractionSource() }
  val pressed by interactionSource.collectIsPressedAsState()
  val scale by
    animateFloatAsState(
      targetValue = if (pressed) ToolbarControlPressedScale else 1f,
      animationSpec = tween(ToolbarPressAnimationMs, easing = EaseOut),
      label = "editor-toolbar-button-scale",
    )
  val surfaceModifier =
    if (surface) {
      Modifier.shadow(AppTheme.shadows.sm, shape)
        .clip(shape)
        .background(AppTheme.colors.surfaceDefault, shape)
        .border(ToolbarBorderWidth, AppTheme.colors.borderEmphasis, shape)
    } else {
      Modifier.clip(shape)
    }

  Box(
    modifier =
      modifier
        .focusProperties { canFocus = false }
        .graphicsLayer {
          scaleX = scale
          scaleY = scale
        }
        .then(surfaceModifier)
        .clickable(interactionSource = interactionSource, indication = null, onClick = onClick),
    contentAlignment = Alignment.Center,
  ) {
    if (crossfadeIcon) {
      Crossfade(
        targetState = icon to contentDescription,
        animationSpec = tween(ToolbarIconCrossfadeMs),
        label = "editor-toolbar-fab-icon-crossfade",
      ) { (targetIcon, targetContentDescription) ->
        EditorToolbarIcon(icon = targetIcon, contentDescription = targetContentDescription)
      }
    } else {
      EditorToolbarIcon(icon = icon, contentDescription = contentDescription)
    }
  }
}

@Composable
private fun EditorToolbarIcon(icon: IconData, contentDescription: String) {
  Icon(
    icon = icon,
    contentDescription = contentDescription,
    modifier = Modifier.size(ToolbarIconSize),
    tint = AppTheme.colors.textDefault,
  )
}
