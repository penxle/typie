package co.typie.screen.editor.editor.overlay

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.SizeTransform
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import co.typie.ext.clickable
import co.typie.ext.horizontalScroll
import co.typie.icons.Lucide
import co.typie.ui.component.Text
import co.typie.ui.component.scrollFog
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.input.hoverFeedback
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
internal fun EditorCompactContextMenu(
  actions: EditorContextMenuActions,
  editorMutationEnabled: Boolean,
  onExpandMenu: () -> Unit,
  acceptsInput: Boolean,
) {
  var expansionVisible by remember { mutableStateOf(false) }
  AnimatedContent(
    targetState = expansionVisible,
    transitionSpec = {
      val direction = if (targetState) 1 else -1
      (slideInHorizontally(tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing)) {
          direction * it / 8
        } + fadeIn(tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing)))
        .togetherWith(
          slideOutHorizontally(tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing)) {
            -direction * it / 8
          } + fadeOut(tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing))
        )
        .using(
          SizeTransform { _, _ ->
            tween(ContextMenuAnimationMillis, easing = ContextMenuEnterEasing)
          }
        )
    },
    label = "EditorContextMenuPage",
  ) { expanded ->
    val items =
      if (expanded) {
        actions.expansionItems().map { item ->
          item.label to
            {
              item.onClick()
              expansionVisible = false
            }
        }
      } else {
        buildList {
          if (actions.showCopyCutActions) {
            add("복사" to actions.onCopy.withDismiss(actions.onDismiss))
            if (editorMutationEnabled) {
              add("잘라내기" to actions.onCut.withDismiss(actions.onDismiss))
            }
          }
          if (editorMutationEnabled) {
            add("붙여넣기" to actions.onPaste.withDismiss(actions.onDismiss))
          }
          if (actions.availableExpansionUnits.isNotEmpty()) {
            add("선택 확장" to { expansionVisible = true })
          }
        }
      }
    val scrollState = rememberScrollState()
    val fogPadding =
      with(LocalDensity.current) {
        PaddingValues(
          start =
            if (scrollState.canScrollBackward) scrollState.value.toDp().coerceAtMost(12.dp)
            else 0.dp,
          end =
            if (scrollState.canScrollForward) {
              (scrollState.maxValue - scrollState.value).toDp().coerceAtMost(12.dp)
            } else 0.dp,
        )
      }
    val pageAcceptsInput = acceptsInput && expanded == expansionVisible
    Row(Modifier.height(IntrinsicSize.Min), verticalAlignment = Alignment.CenterVertically) {
      if (expanded) {
        EditorCompactContextMenuIconItem(Lucide.ChevronLeft, "이전", pageAcceptsInput) {
          expansionVisible = false
        }
      }
      Row(
        Modifier.weight(1f, fill = false)
          .scrollFog(fogPadding, AppTheme.colors.surfaceDefault)
          .horizontalScroll(scrollState, enabled = pageAcceptsInput),
        verticalAlignment = Alignment.CenterVertically,
      ) {
        items.forEachIndexed { index, (label, onClick) ->
          EditorCompactContextMenuItem(
            label = label,
            acceptsInput = pageAcceptsInput,
            contentPadding =
              PaddingValues(
                // Keep space at the outer edge; the chevrons already include optical padding.
                start =
                  if (index == 0) {
                    if (expanded) 0.dp else 16.dp
                  } else 10.dp,
                end = if (index == items.lastIndex) 0.dp else 10.dp,
                top = 12.dp,
                bottom = 12.dp,
              ),
            onClick = onClick,
          )
        }
      }
      EditorCompactContextMenuIconItem(
        Lucide.ChevronRight,
        "메뉴 펼치기",
        pageAcceptsInput,
        onExpandMenu,
      )
    }
  }
}

@Composable
private fun EditorCompactContextMenuIconItem(
  icon: IconData,
  description: String,
  acceptsInput: Boolean,
  onClick: () -> Unit,
) {
  val interactionSource = remember { MutableInteractionSource() }
  Box(
    Modifier.width(48.dp)
      .heightIn(min = 48.dp)
      .fillMaxHeight()
      .hoverFeedback(interactionSource, enabled = acceptsInput, shape = ContextMenuShape)
      .clickable(enabled = acceptsInput, interactionSource = interactionSource) { onClick() },
    contentAlignment = Alignment.Center,
  ) {
    Icon(
      icon,
      contentDescription = description,
      modifier = Modifier.size(18.dp),
      tint = AppTheme.colors.textDefault,
    )
  }
}

@Composable
private fun EditorCompactContextMenuItem(
  label: String,
  acceptsInput: Boolean,
  contentPadding: PaddingValues,
  onClick: () -> Unit,
) {
  val interactionSource = remember { MutableInteractionSource() }
  Box(
    Modifier.widthIn(min = 48.dp)
      .fillMaxHeight()
      .hoverFeedback(interactionSource, enabled = acceptsInput, shape = ContextMenuShape)
      .clickable(enabled = acceptsInput, interactionSource = interactionSource) { onClick() }
      .padding(contentPadding),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      label,
      style = AppTheme.typography.action,
      color = AppTheme.colors.textDefault,
      maxLines = 1,
      softWrap = false,
    )
  }
}
