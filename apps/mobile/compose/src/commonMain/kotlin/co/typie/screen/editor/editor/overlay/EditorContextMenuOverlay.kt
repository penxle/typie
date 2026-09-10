package co.typie.screen.editor.editor.overlay

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateBounds
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.rememberTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathOperation
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.addOutline
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.LookaheadScope
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import androidx.compose.ui.util.lerp
import co.typie.editor.Editor
import co.typie.editor.ext.isCollapsed
import co.typie.editor.runtime.EditorContextMenuMode
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ext.clickable
import co.typie.navigation.LocalNavigationForegroundInteractive
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PressGestureSession
import co.typie.ui.input.WindowInputHandler
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import co.typie.ui.utils.matchesShortcut
import kotlin.math.max
import kotlin.math.roundToInt

internal val ContextMenuShape = AppShapes.squircle(AppShapes.lg)
private val ContextMenuEdgePadding = 4.dp
private val ContextMenuGap = 24.dp
internal val ContextMenuEnterEasing = CubicBezierEasing(0.215f, 0.61f, 0.355f, 1f)
internal const val ContextMenuAnimationMillis = 150
private const val ContextSubmenuAnimationMillis = 240

@Composable
internal fun EditorSelectionContextMenuOverlay(
  anchor: EditorContextMenuAnchor,
  overlaySize: Size,
  visibleArea: EditorVisibleArea,
  actions: EditorContextMenuActions,
  editorMutationEnabled: Boolean = true,
  mode: EditorContextMenuMode,
  onExpandMenu: () -> Unit,
  onBoundsInWindowChanged: (List<Rect>) -> Unit = {},
  visible: Boolean = true,
  onHidden: () -> Unit = {},
) {
  val reportBounds by rememberUpdatedState(onBoundsInWindowChanged)
  val completeExit by rememberUpdatedState(onHidden)
  val exitProgress = remember { Animatable(0f) }
  LaunchedEffect(visible) {
    if (visible) {
      exitProgress.snapTo(0f)
    } else {
      exitProgress.animateTo(1f, tween(120, easing = LinearEasing))
      completeExit()
    }
  }
  var menuBounds by remember { mutableStateOf<Rect?>(null) }
  var submenuBounds by remember { mutableStateOf<Rect?>(null) }
  var selectionItemBounds by remember { mutableStateOf<Rect?>(null) }
  var origin by remember { mutableStateOf(Offset.Zero) }
  val enterState = remember { MutableTransitionState(false) }
  enterState.targetState = true
  val submenu = remember(mode) { MutableTransitionState(false) }
  var pressGestureSession by remember { mutableStateOf<PressGestureSession?>(null) }
  val parentSuspended = submenu.currentState || submenu.targetState || !submenu.isIdle
  val closeTopMenu = {
    if (submenu.targetState) submenu.targetState = false else actions.onDismiss()
  }
  PlatformBackHandler(enabled = visible, onBack = closeTopMenu)
  WindowInputHandler(
    enabled = visible && LocalNavigationForegroundInteractive.current,
    onKeyEvent = { event ->
      if (matchesShortcut(event, Key.Escape)) {
        closeTopMenu()
        true
      } else false
    },
  )
  val submenuTransition = rememberTransition(submenu, label = "EditorContextSubmenu")
  val submenuProgress by
    submenuTransition.animateFloat(
      transitionSpec = { tween(ContextSubmenuAnimationMillis, easing = ContextMenuEnterEasing) },
      label = "reveal",
    ) {
      if (it) 1f else 0f
    }
  val parentScale = 1f - 0.03f * submenuProgress
  val parentOpacity = 1f - 0.55f * submenuProgress
  val shape =
    if (mode == EditorContextMenuMode.Expanded) {
      AppShapes.squircle(PopoverDefaults.ExpandedRadius)
    } else ContextMenuShape
  val inputBounds = buildList {
    menuBounds?.let { bounds ->
      val insetX = bounds.width * (1f - parentScale) / 2f
      val insetY = bounds.height * (1f - parentScale) / 2f
      add(
        Rect(
          bounds.left + insetX,
          bounds.top + insetY,
          bounds.right - insetX,
          bounds.bottom - insetY,
        )
      )
    }
    submenuBounds?.let(::add)
  }
  SideEffect { reportBounds(if (visible) inputBounds else emptyList()) }
  DisposableEffect(Unit) { onDispose { reportBounds(emptyList()) } }

  val borderColor = AppTheme.colors.borderDefault
  Box(
    Modifier.fillMaxSize()
      .then(if (visible) Modifier else Modifier.clearAndSetSemantics {})
      .graphicsLayer { alpha = 1f - exitProgress.value }
      .drawWithContent {
        val exitScale = 1f - 0.04f * EaseOutCubic.transform(exitProgress.value)
        val pivot = menuBounds?.center?.minus(origin) ?: center
        // Draw the entire stack together without changing the measured paths or input bounds.
        scale(exitScale, pivot = pivot) { this@drawWithContent.drawContent() }
      }
      .onGloballyPositioned { origin = it.positionInWindow() }
      .drawWithCache {
        val parent = menuBounds
        val child = submenuBounds
        if (!parentSuspended || parent == null || child == null) {
          onDrawWithContent { drawContent() }
        } else {
          fun menuPath(bounds: Rect, scale: Float = 1f): Path {
            val inset = Offset(bounds.width, bounds.height) * ((1f - scale) / 2f)
            val menuShape = AppShapes.squircle(PopoverDefaults.ExpandedRadius * scale)
            return Path().apply {
              addOutline(
                menuShape.createOutline(bounds.size * scale, layoutDirection, this@drawWithCache)
              )
              translate(bounds.topLeft + inset - origin)
            }
          }
          val parentPath = menuPath(parent, parentScale)
          val childPath = menuPath(child)
          val outerPath = Path.combine(PathOperation.Union, parentPath, childPath)
          val borderStroke = Stroke(2 * 1.dp.toPx())
          onDrawWithContent {
            drawContent()
            // Keep the combined silhouette solid. Only the new boundary inside the parent fades.
            clipPath(parentPath) {
              clipPath(childPath) {
                drawPath(childPath, borderColor, alpha = submenuProgress, style = borderStroke)
              }
            }
            clipPath(outerPath) { drawPath(outerPath, borderColor, style = borderStroke) }
          }
        }
      }
  ) {
    // Resolve the destination before animating, so growing menus cannot flip sides midway.
    LookaheadScope {
      EditorContextMenuLayout(anchor, overlaySize, visibleArea) {
        AnimatedVisibility(
          visibleState = enterState,
          enter =
            fadeIn(tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing)) +
              scaleIn(
                initialScale = 0.8f,
                animationSpec = tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing),
              ),
        ) {
          AnimatedContent(
            targetState = mode,
            modifier =
              Modifier.animateBounds(
                  lookaheadScope = this@LookaheadScope,
                  boundsTransform = { _, _ ->
                    tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing)
                  },
                )
                .onGloballyPositioned { menuBounds = it.boundsInWindow() },
            transitionSpec = {
              fadeIn(tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing))
                .togetherWith(
                  fadeOut(tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing))
                )
                .using(null)
            },
            contentAlignment = Alignment.TopStart,
            label = "EditorContextMenuMode",
          ) { targetMode ->
            Box(
              Modifier.graphicsLayer {
                scaleX = parentScale
                scaleY = parentScale
              }
            ) {
              EditorContextMenuSurface(shape, drawBorder = !parentSuspended) {
                val contentModifier =
                  if (parentSuspended) Modifier.clearAndSetSemantics {} else Modifier
                Box(contentModifier.graphicsLayer { alpha = parentOpacity }) {
                  if (targetMode == EditorContextMenuMode.Compact) {
                    EditorCompactContextMenu(
                      actions,
                      editorMutationEnabled,
                      onExpandMenu,
                      acceptsInput = visible,
                    )
                  } else {
                    EditorExpandedContextMenu(
                      actions,
                      editorMutationEnabled,
                      acceptsInput = visible && !parentSuspended,
                      onSelectionMenuOpen = {
                        pressGestureSession = null
                        submenu.targetState = true
                      },
                      onSelectionPressSession = { pressGestureSession = it },
                      onSelectionItemBoundsChanged = {
                        if (!parentSuspended) selectionItemBounds = it
                      },
                    )
                  }
                }
                if (parentSuspended) {
                  Box(
                    Modifier.matchParentSize()
                      .semantics { contentDescription = "상위 메뉴로 돌아가기" }
                      .clickable(enabled = visible) { submenu.targetState = false }
                  )
                }
              }
            }
          }
        }
      }
    }
    val parent = menuBounds
    val item = selectionItemBounds
    if (parentSuspended && parent != null && item != null) {
      val panePadding = with(LocalDensity.current) { PopoverDefaults.PanePadding.toPx() }
      EditorContextMenuLayout(
        anchor,
        overlaySize,
        visibleArea,
        preferredTopLeft = Offset(parent.left - origin.x, item.top - origin.y - panePadding),
        revealFrom =
          Rect(
            left = parent.left - origin.x,
            top = item.top - origin.y,
            right = parent.right - origin.x,
            bottom = item.bottom - origin.y,
          ),
        revealProgress = submenuProgress,
      ) {
        EditorContextMenuSurface(
          shape = AppShapes.squircle(PopoverDefaults.ExpandedRadius),
          drawBorder = false,
          shadowAlpha = submenuProgress,
          modifier = Modifier.onGloballyPositioned { submenuBounds = it.boundsInWindow() },
        ) {
          DisposableEffect(Unit) { onDispose { submenuBounds = null } }
          // The initial header occupies exactly the trigger row; padding unfolds with the card.
          Box(Modifier.graphicsLayer { translationY = -panePadding * (1f - submenuProgress) }) {
            EditorSelectionContextSubmenu(
              actions = actions,
              acceptsInput = visible && submenu.targetState,
              pressGestureSession = pressGestureSession,
              revealProgress = submenuProgress,
              onCollapse = { submenu.targetState = false },
            )
          }
        }
      }
    }
  }
}

@Composable
private fun EditorContextMenuSurface(
  shape: Shape,
  modifier: Modifier = Modifier,
  drawBorder: Boolean = true,
  shadowAlpha: Float = 1f,
  content: @Composable androidx.compose.foundation.layout.BoxScope.() -> Unit,
) {
  Box(
    modifier
      .shadow(AppTheme.shadows.md, shape, alpha = { shadowAlpha })
      .clip(shape)
      .then(
        if (drawBorder) Modifier.border(1.dp, AppTheme.colors.borderDefault, shape) else Modifier
      )
      .background(AppTheme.colors.surfaceDefault, shape),
    content = content,
  )
}

@Composable
private fun EditorContextMenuLayout(
  anchor: EditorContextMenuAnchor,
  overlaySize: Size,
  visibleArea: EditorVisibleArea,
  preferredTopLeft: Offset? = null,
  revealFrom: Rect? = null,
  revealProgress: Float = 1f,
  content: @Composable () -> Unit,
) {
  val density = LocalDensity.current

  Layout(modifier = Modifier.fillMaxSize(), content = content) { measurables, constraints ->
    val padding = (ContextMenuEdgePadding.value * density.density).roundToInt()
    val visibleTop =
      (visibleArea.visibleViewportTop * density.density).coerceIn(0f, overlaySize.height)
    val visibleBottom =
      (visibleArea.visibleViewportBottom * density.density).coerceIn(visibleTop, overlaySize.height)
    val contentConstraints =
      constraints.copy(
        minWidth = 0,
        minHeight = 0,
        maxWidth =
          minOf(
            constraints.maxWidth,
            (overlaySize.width.roundToInt() - padding * 2).coerceAtLeast(0),
          ),
        maxHeight =
          minOf(
            constraints.maxHeight,
            ((visibleBottom - visibleTop).roundToInt() - padding * 2).coerceAtLeast(0),
          ),
      )
    val measurable = measurables.single()
    val rootPlaceable = if (revealFrom == null) measurable.measure(contentConstraints) else null
    // Measure the destination independently of the reveal, so viewport clamping cannot jump
    // ahead of the animation. Only the surface height and position change; its width is fixed.
    val menuSize =
      if (rootPlaceable != null) {
        Size(rootPlaceable.width.toFloat(), rootPlaceable.height.toFloat())
      } else {
        val width =
          measurable
            .maxIntrinsicWidth(contentConstraints.maxHeight)
            .coerceIn(0, contentConstraints.maxWidth)
        val height = measurable.maxIntrinsicHeight(width).coerceIn(0, contentConstraints.maxHeight)
        Size(width.toFloat(), height.toFloat())
      }
    val placement =
      resolveEditorContextMenuPlacement(
        anchor = anchor,
        menuSize = menuSize,
        overlaySize = overlaySize,
        visibleArea = visibleArea,
        density = density.density,
        preferredTopLeft = preferredTopLeft,
      )

    val placeable =
      rootPlaceable
        ?: measurable.measure(
          Constraints.fixed(
            width = menuSize.width.roundToInt(),
            height =
              lerp(requireNotNull(revealFrom).height, menuSize.height, revealProgress)
                .roundToInt()
                .coerceIn(0, contentConstraints.maxHeight),
          )
        )
    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      if (placement != null) {
        val position =
          if (revealFrom == null) placement.topLeft
          else
            Offset(
              lerp(revealFrom.left, placement.topLeft.x, revealProgress),
              lerp(revealFrom.top, placement.topLeft.y, revealProgress),
            )
        placeable.place(x = position.x.roundToInt(), y = position.y.roundToInt())
      }
    }
  }
}

internal data class EditorContextMenuAnchor(
  val centerX: Float,
  val above: Float,
  val below: Float,
  val atPointer: Boolean = false,
)

internal data class EditorContextMenuPlacement(val topLeft: Offset)

internal fun resolveEditorContextMenuPlacement(
  anchor: EditorContextMenuAnchor,
  menuSize: Size,
  overlaySize: Size,
  visibleArea: EditorVisibleArea,
  density: Float,
  preferredTopLeft: Offset? = null,
): EditorContextMenuPlacement? {
  if (
    density <= 0f ||
      menuSize.width <= 0f ||
      menuSize.height <= 0f ||
      overlaySize.width <= 0f ||
      overlaySize.height <= 0f
  ) {
    return null
  }

  val edgePaddingPx = ContextMenuEdgePadding.value * density
  val visibleTopPx = (visibleArea.visibleViewportTop * density).coerceIn(0f, overlaySize.height)
  val visibleBottomPx =
    (visibleArea.visibleViewportBottom * density).coerceIn(visibleTopPx, overlaySize.height)
  val canShowAbove = anchor.above - visibleTopPx >= menuSize.height
  val canShowBelow = visibleBottomPx - anchor.below >= menuSize.height
  val centerInVisibleArea = !canShowAbove && !canShowBelow
  val maxLeft = max(edgePaddingPx, overlaySize.width - menuSize.width - edgePaddingPx)
  val preferredLeft =
    if (anchor.atPointer) {
      anchor.centerX
    } else if (centerInVisibleArea) {
      (overlaySize.width - menuSize.width) / 2f
    } else {
      anchor.centerX - menuSize.width / 2f
    }
  val left = (preferredTopLeft?.x ?: preferredLeft).coerceIn(edgePaddingPx, maxLeft)
  val preferredTop =
    when {
      anchor.atPointer && canShowBelow -> anchor.below
      canShowAbove -> anchor.above - menuSize.height
      canShowBelow -> anchor.below
      else -> visibleTopPx + (visibleBottomPx - visibleTopPx - menuSize.height) / 2f
    }
  val minTop = visibleTopPx + edgePaddingPx
  val maxTop = max(minTop, visibleBottomPx - menuSize.height - edgePaddingPx)
  val top = (preferredTopLeft?.y ?: preferredTop).coerceIn(minTop, maxTop)

  return EditorContextMenuPlacement(topLeft = Offset(left, top))
}

internal fun resolveContextMenuAnchor(
  editor: Editor,
  uiState: EditorUiState,
  editorRectInOverlay: Rect,
  density: Float,
): EditorContextMenuAnchor? {
  if (density <= 0f) {
    return null
  }

  val transform = uiState.resolveViewportTransform(pageSizes = editor.publishedState.pageSizes)
  uiState.contextMenu.pointerPosition?.let { point ->
    val position =
      transform.localToGlobal(page = point.page, x = point.x, y = point.y) ?: return null
    val x = editorRectInOverlay.left + position.x * density
    val y = editorRectInOverlay.top + position.y * density
    return EditorContextMenuAnchor(centerX = x, above = y, below = y, atPointer = true)
  }
  val rangeSelection = editor.publishedState.selection?.takeIf { !it.isCollapsed() }
  val gapPx = ContextMenuGap.value * density

  if (rangeSelection != null) {
    val endpoints = editor.publishedState.selectionEndpoints ?: return null
    val fromRect = endpoints.from.rect
    val toRect = endpoints.to.rect
    val from =
      transform.localToGlobal(page = endpoints.from.pageIdx, x = fromRect.x, y = fromRect.y)
        ?: return null
    val to =
      transform.localToGlobal(
        page = endpoints.to.pageIdx,
        x = toRect.x,
        y = toRect.y + toRect.height,
      ) ?: return null
    val topY = editorRectInOverlay.top + from.y * density
    val bottomY = editorRectInOverlay.top + to.y * density
    return EditorContextMenuAnchor(
      centerX = editorRectInOverlay.left + ((from.x + to.x) / 2f) * density,
      above = topY - gapPx,
      below = bottomY + gapPx,
    )
  }

  val cursor = editor.publishedState.cursor ?: return null
  val caret = cursor.caret
  val top = transform.localToGlobal(page = cursor.pageIdx, x = caret.x, y = caret.y) ?: return null
  val bottom =
    transform.localToGlobal(page = cursor.pageIdx, x = caret.x, y = caret.y + caret.height)
      ?: return null

  return EditorContextMenuAnchor(
    centerX = editorRectInOverlay.left + top.x * density,
    above = editorRectInOverlay.top + top.y * density - gapPx,
    below = editorRectInOverlay.top + bottom.y * density + gapPx,
  )
}
