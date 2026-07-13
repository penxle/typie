package co.typie.screen.editor.editor.overlay

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.editor.Editor
import co.typie.editor.ext.isCollapsed
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.editor.runtime.EditorUiState
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.ext.clickable
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.icon.Icon
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import co.typie.ui.theme.shadow
import kotlin.math.max
import kotlin.math.roundToInt

private val ContextMenuShape = AppShapes.rounded(8.dp)
private val ContextMenuEdgePadding = 4.dp
private val ContextMenuGap = 24.dp
private val ContextMenuEnterEasing = CubicBezierEasing(0.215f, 0.61f, 0.355f, 1f)
private const val ContextMenuAnimationMillis = 150

private enum class EditorContextMenuPage {
  Primary,
  Expansion,
}

@Composable
internal fun EditorSelectionContextMenuOverlay(
  anchor: EditorContextMenuAnchor,
  overlaySize: Size,
  visibleArea: EditorVisibleArea,
  showCopyCutActions: Boolean,
  availableExpansionUnits: Set<SelectionExpansionUnit>,
  onCopy: () -> Unit,
  onCut: () -> Unit,
  onPaste: () -> Unit,
  onExpandWord: () -> Unit,
  onExpandSentence: () -> Unit,
  onExpandParagraph: () -> Unit,
  onSelectAll: () -> Unit,
  onDismiss: () -> Unit,
) {
  val enterState = remember { MutableTransitionState(false) }
  enterState.targetState = true
  var page by remember { mutableStateOf(EditorContextMenuPage.Primary) }
  val expansionItems =
    listOf(
        EditorContextMenuExpansionItem("단어", SelectionExpansionUnit.Word, onExpandWord),
        EditorContextMenuExpansionItem("문장", SelectionExpansionUnit.Sentence, onExpandSentence),
        EditorContextMenuExpansionItem("문단", SelectionExpansionUnit.Paragraph, onExpandParagraph),
        EditorContextMenuExpansionItem("전체", SelectionExpansionUnit.All, onSelectAll),
      )
      .filter { it.unit in availableExpansionUnits }

  EditorContextMenuLayout(anchor = anchor, overlaySize = overlaySize, visibleArea = visibleArea) {
    AnimatedVisibility(
      visibleState = enterState,
      enter =
        fadeIn(animationSpec = tween(durationMillis = 150, easing = ContextMenuEnterEasing)) +
          scaleIn(
            initialScale = 0.8f,
            animationSpec = tween(durationMillis = 150, easing = ContextMenuEnterEasing),
          ),
    ) {
      AnimatedContent(
        targetState = page,
        modifier =
          Modifier.shadow(AppTheme.shadows.md, ContextMenuShape)
            .clip(ContextMenuShape)
            .border(1.dp, AppTheme.colors.borderDefault, ContextMenuShape)
            .background(AppTheme.colors.surfaceDefault, ContextMenuShape),
        transitionSpec = {
          val direction = if (targetState == EditorContextMenuPage.Expansion) 1 else -1
          (slideInHorizontally(
              animationSpec =
                tween(durationMillis = ContextMenuAnimationMillis, easing = ContextMenuEnterEasing),
              initialOffsetX = { direction * it / 8 },
            ) +
              fadeIn(
                animationSpec =
                  tween(
                    durationMillis = ContextMenuAnimationMillis,
                    easing = ContextMenuEnterEasing,
                  )
              ))
            .togetherWith(
              slideOutHorizontally(
                animationSpec =
                  tween(
                    durationMillis = ContextMenuAnimationMillis,
                    easing = ContextMenuEnterEasing,
                  ),
                targetOffsetX = { -direction * it / 8 },
              ) +
                fadeOut(
                  animationSpec =
                    tween(
                      durationMillis = ContextMenuAnimationMillis,
                      easing = ContextMenuEnterEasing,
                    )
                )
            )
            .using(
              SizeTransform { _, _ ->
                tween(durationMillis = ContextMenuAnimationMillis, easing = ContextMenuEnterEasing)
              }
            )
        },
        contentAlignment = Alignment.CenterStart,
        label = "EditorSelectionContextMenuPage",
      ) { targetPage ->
        Row(
          horizontalArrangement = Arrangement.Center,
          verticalAlignment = Alignment.CenterVertically,
        ) {
          when (targetPage) {
            EditorContextMenuPage.Primary -> {
              if (showCopyCutActions) {
                EditorContextMenuItem(label = "복사", onClick = onCopy.withDismiss(onDismiss))
                EditorContextMenuItem(label = "잘라내기", onClick = onCut.withDismiss(onDismiss))
              }
              EditorContextMenuItem(label = "붙여넣기", onClick = onPaste.withDismiss(onDismiss))
              if (availableExpansionUnits.isNotEmpty()) {
                EditorContextMenuItem(
                  label = "선택 확장",
                  onClick = { page = EditorContextMenuPage.Expansion },
                )
              }
            }
            EditorContextMenuPage.Expansion -> {
              EditorContextMenuBackItem(onClick = { page = EditorContextMenuPage.Primary })
              expansionItems.forEach { item ->
                EditorContextMenuItem(
                  label = item.label,
                  onClick = {
                    item.onClick()
                    page = EditorContextMenuPage.Primary
                  },
                )
              }
            }
          }
        }
      }
    }
  }
}

private data class EditorContextMenuExpansionItem(
  val label: String,
  val unit: SelectionExpansionUnit,
  val onClick: () -> Unit,
)

private fun (() -> Unit).withDismiss(onDismiss: () -> Unit): () -> Unit = {
  this()
  onDismiss()
}

@Composable
private fun EditorContextMenuLayout(
  anchor: EditorContextMenuAnchor,
  overlaySize: Size,
  visibleArea: EditorVisibleArea,
  content: @Composable () -> Unit,
) {
  val density = LocalDensity.current

  Layout(modifier = Modifier.fillMaxSize(), content = content) { measurables, constraints ->
    val placeable = measurables.single().measure(constraints.copy(minWidth = 0, minHeight = 0))
    val placement =
      resolveEditorContextMenuPlacement(
        anchor = anchor,
        menuSize = Size(width = placeable.width.toFloat(), height = placeable.height.toFloat()),
        overlaySize = overlaySize,
        visibleArea = visibleArea,
        density = density.density,
      )

    layout(width = constraints.maxWidth, height = constraints.maxHeight) {
      if (placement != null) {
        placeable.place(x = placement.topLeft.x.roundToInt(), y = placement.topLeft.y.roundToInt())
      }
    }
  }
}

@Composable
private fun EditorContextMenuBackItem(onClick: () -> Unit) {
  Box(
    modifier = Modifier.clickable { onClick() }.padding(horizontal = 14.dp, vertical = 8.dp),
    contentAlignment = Alignment.Center,
  ) {
    Icon(
      icon = Lucide.ChevronLeft,
      contentDescription = "이전",
      modifier = Modifier.size(14.dp),
      tint = AppTheme.colors.textDefault,
    )
  }
}

@Composable
private fun EditorContextMenuItem(label: String, onClick: () -> Unit = {}) {
  Text(
    text = label,
    style = AppTheme.typography.caption,
    color = AppTheme.colors.textDefault,
    modifier = Modifier.clickable { onClick() }.padding(horizontal = 12.dp, vertical = 10.dp),
  )
}

internal data class EditorContextMenuAnchor(val centerX: Float, val above: Float, val below: Float)

internal data class EditorContextMenuPlacement(val topLeft: Offset)

internal fun resolveEditorContextMenuPlacement(
  anchor: EditorContextMenuAnchor,
  menuSize: Size,
  overlaySize: Size,
  visibleArea: EditorVisibleArea,
  density: Float,
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
    if (centerInVisibleArea) {
      (overlaySize.width - menuSize.width) / 2f
    } else {
      anchor.centerX - menuSize.width / 2f
    }
  val left = preferredLeft.coerceIn(edgePaddingPx, maxLeft)
  val preferredTop =
    when {
      canShowAbove -> anchor.above - menuSize.height
      canShowBelow -> anchor.below
      else -> visibleTopPx + (visibleBottomPx - visibleTopPx - menuSize.height) / 2f
    }
  val minTop = visibleTopPx + edgePaddingPx
  val maxTop = max(minTop, visibleBottomPx - menuSize.height - edgePaddingPx)
  val top = preferredTop.coerceIn(minTop, maxTop)

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

  val transform = uiState.resolveViewportTransform(pageSizes = editor.pageSizes)
  val rangeSelection = editor.selection?.takeIf { !it.isCollapsed() }
  val gapPx = ContextMenuGap.value * density

  if (rangeSelection != null) {
    val endpoints = editor.tickSelectionEndpoints ?: return null
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

  val cursor = editor.cursor ?: return null
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
