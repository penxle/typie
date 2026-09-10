package co.typie.screen.editor.editor.overlay

import androidx.compose.foundation.background
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.boundsInWindow
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.unit.dp
import co.typie.editor.ffi.SelectionExpansionUnit
import co.typie.ext.edgeAutoScroll
import co.typie.ext.rememberEdgeAutoScrollController
import co.typie.ext.verticalScroll
import co.typie.icons.Lucide
import co.typie.icons.Typie
import co.typie.ui.component.popover.LocalPopoverPaneEdgeAutoScrollController
import co.typie.ui.component.popover.PopoverDefaults
import co.typie.ui.component.popover.PopoverList
import co.typie.ui.component.popover.PopoverListItem
import co.typie.ui.component.popover.PopoverMenuItemRow
import co.typie.ui.component.popover.PressGestureSession
import co.typie.ui.component.popover.SelectablePaneHost
import co.typie.ui.component.popover.rememberPopoverTriggerInputModifier
import co.typie.ui.icon.Icon
import co.typie.ui.icon.IconData
import co.typie.ui.state.rememberScrollState
import co.typie.ui.theme.AppTheme

@Composable
internal fun EditorExpandedContextMenu(
  actions: EditorContextMenuActions,
  editorMutationEnabled: Boolean,
  acceptsInput: Boolean,
  onSelectionMenuOpen: () -> Unit,
  onSelectionPressSession: (PressGestureSession?) -> Unit,
  onSelectionItemBoundsChanged: (Rect) -> Unit,
) {
  val clipboardItems = buildList {
    if (actions.showCopyCutActions) {
      add(
        PopoverListItem(
          content = { PopoverMenuItemRow(Lucide.Copy, "복사") },
          onSelected = actions.onCopy.withDismiss(actions.onDismiss),
        )
      )
      if (editorMutationEnabled) {
        add(
          PopoverListItem(
            content = { PopoverMenuItemRow(Lucide.Scissors, "잘라내기") },
            onSelected = actions.onCut.withDismiss(actions.onDismiss),
          )
        )
      }
    }
    if (editorMutationEnabled) {
      add(
        PopoverListItem(
          content = { PopoverMenuItemRow(Lucide.ClipboardPaste, "붙여넣기") },
          onSelected = actions.onPaste.withDismiss(actions.onDismiss),
        )
      )
    }
  }
  EditorContextMenuPane(acceptsInput) {
    PopoverList(clipboardItems, acceptsInput = acceptsInput)
    if (actions.availableExpansionUnits.isNotEmpty()) {
      if (clipboardItems.isNotEmpty()) EditorContextMenuDivider()
      var itemCoordinates by remember { mutableStateOf<LayoutCoordinates?>(null) }
      val interactionSource = remember { MutableInteractionSource() }
      val triggerInput =
        rememberPopoverTriggerInputModifier(
          interactionSource = interactionSource,
          canOpen = { acceptsInput },
          positionInWindow = { position ->
            itemCoordinates?.takeIf { it.isAttached }?.localToWindow(position)
          },
          onOpen = onSelectionMenuOpen,
          onSession = onSelectionPressSession,
        )
      Box(
        Modifier.onGloballyPositioned {
            itemCoordinates = it
            onSelectionItemBoundsChanged(it.boundsInWindow())
          }
          .then(triggerInput)
      ) {
        PopoverList(
          acceptsInput = acceptsInput,
          items =
            listOf(
              PopoverListItem(
                content = { EditorContextMenuSelectionRow(expanded = false) },
                onSelected = onSelectionMenuOpen,
              )
            ),
        )
      }
    }
    if (actions.contextualItems.isNotEmpty() || actions.onComment != null) {
      if (clipboardItems.isNotEmpty() || actions.availableExpansionUnits.isNotEmpty()) {
        EditorContextMenuDivider()
      }
      PopoverList(
        acceptsInput = acceptsInput,
        items =
          actions.contextualItems.map { item ->
            PopoverListItem(
              content = {
                Box(item.modifier) {
                  PopoverMenuItemRow(
                    item.icon,
                    item.label,
                    color = if (item.destructive) AppTheme.colors.danger else null,
                  )
                }
              },
              onSelected = item.onClick.withDismiss(actions.onDismiss),
            )
          },
      )
      actions.onComment?.let { onComment ->
        PopoverList(
          acceptsInput = acceptsInput,
          items =
            listOf(
              PopoverListItem(
                content = { PopoverMenuItemRow(Lucide.MessageSquarePlus, "코멘트 달기") },
                onSelected = onComment.withDismiss(actions.onDismiss),
              )
            ),
        )
      }
    }
  }
}

@Composable
internal fun EditorSelectionContextSubmenu(
  actions: EditorContextMenuActions,
  acceptsInput: Boolean,
  pressGestureSession: PressGestureSession?,
  revealProgress: Float,
  onCollapse: () -> Unit,
) {
  EditorContextMenuPane(acceptsInput, pressGestureSession) {
    PopoverList(
      acceptsInput = acceptsInput,
      items =
        listOf(
          PopoverListItem(
            content = { EditorContextMenuSelectionRow(expanded = true, progress = revealProgress) },
            onSelected = onCollapse,
          )
        ),
    )
    Column(Modifier.graphicsLayer { alpha = revealProgress }) {
      EditorContextMenuDivider()
      PopoverList(
        acceptsInput = acceptsInput,
        items =
          actions.expansionItems().map { item ->
            PopoverListItem(
              content = { PopoverMenuItemRow(item.icon, "${item.label} 선택") },
              onSelected = {
                onCollapse()
                item.onClick()
              },
            )
          },
      )
    }
  }
}

@Composable
private fun EditorContextMenuSelectionRow(
  expanded: Boolean,
  progress: Float = if (expanded) 1f else 0f,
) {
  Row(verticalAlignment = Alignment.CenterVertically) {
    PopoverMenuItemRow(Lucide.TextSelect, "선택 확장")
    Spacer(Modifier.weight(1f))
    Icon(
      Lucide.ChevronRight,
      contentDescription = if (expanded) "선택 확장 접기" else null,
      modifier =
        Modifier.padding(end = 16.dp).size(16.dp).graphicsLayer { rotationZ = 90f * progress },
      tint = AppTheme.colors.textDefault,
    )
  }
}

@Composable
private fun EditorContextMenuPane(
  acceptsInput: Boolean,
  pressGestureSession: PressGestureSession? = null,
  content: @Composable () -> Unit,
) {
  val scrollState = rememberScrollState()
  val scrollController = rememberEdgeAutoScrollController(verticalScrollableState = scrollState)
  CompositionLocalProvider(LocalPopoverPaneEdgeAutoScrollController provides scrollController) {
    Box(
      Modifier.width(240.dp)
        .edgeAutoScroll(scrollController)
        .verticalScroll(scrollState, enabled = acceptsInput)
    ) {
      SelectablePaneHost(acceptsInput, pressGestureSession) {
        Column(Modifier.padding(PopoverDefaults.PanePadding)) { content() }
      }
    }
  }
}

@Composable
private fun EditorContextMenuDivider() {
  Box(
    Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
      .fillMaxWidth()
      .height(1.dp)
      .background(AppTheme.colors.borderHairline)
  )
}

internal data class EditorContextMenuExpansionItem(
  val label: String,
  val unit: SelectionExpansionUnit,
  val icon: IconData,
  val onClick: () -> Unit,
)

internal fun EditorContextMenuActions.expansionItems() =
  listOf(
      EditorContextMenuExpansionItem(
        "단어",
        SelectionExpansionUnit.Word,
        Typie.TextWord,
        onExpandWord,
      ),
      EditorContextMenuExpansionItem(
        "문장",
        SelectionExpansionUnit.Sentence,
        Typie.TextSentence,
        onExpandSentence,
      ),
      EditorContextMenuExpansionItem(
        "문단",
        SelectionExpansionUnit.Paragraph,
        Lucide.Pilcrow,
        onExpandParagraph,
      ),
      EditorContextMenuExpansionItem(
        "전체",
        SelectionExpansionUnit.All,
        Lucide.SquareDashed,
        onSelectAll,
      ),
    )
    .filter { it.unit in availableExpansionUnits }

internal fun (() -> Unit).withDismiss(onDismiss: () -> Unit): () -> Unit = {
  this()
  onDismiss()
}
