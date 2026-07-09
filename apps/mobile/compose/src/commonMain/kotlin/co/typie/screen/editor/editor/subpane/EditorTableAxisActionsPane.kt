package co.typie.screen.editor.editor.subpane

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import co.typie.editor.EditorTheme
import co.typie.editor.EditorValues
import co.typie.editor.currentEditorThemeVariant
import co.typie.editor.ffi.Axis
import co.typie.editor.ffi.BackgroundColorValue
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.TableOp
import co.typie.editor.ffi.Tri
import co.typie.icons.Lucide
import co.typie.navigation.PlatformBackHandler
import co.typie.ui.component.Divider
import co.typie.ui.component.Text
import co.typie.ui.component.editorsettings.EditorSettingsSwatchRow
import co.typie.ui.component.sheet.AnchoredSheetSurface
import co.typie.ui.component.sheet.SheetActionRow
import co.typie.ui.component.sheet.SheetLayout
import co.typie.ui.component.sheet.SheetPadding
import co.typie.ui.component.sheet.SheetStop
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme

private val TableAxisActionsSheetHorizontalPadding = 24.dp
private val TableAxisActionsSheetActionPadding =
  PaddingValues(horizontal = TableAxisActionsSheetHorizontalPadding, vertical = 8.dp)
private val TableAxisActionsSheetRowPadding = PaddingValues(vertical = 12.dp)

@Composable
internal fun EditorTableAxisActionsPane(
  pane: EditorSubPane.TableAxisActions,
  currentBackgroundColor: Tri<BackgroundColorValue>?,
  dismissRequestVersion: Int,
  onAction: (Message) -> Unit,
  onDismissStarted: () -> Unit,
  onDismiss: () -> Unit,
  onLayoutInfoChanged: (EditorSubPaneLayoutInfo) -> Unit,
  onLayoutInfoCleared: (EditorSubPane) -> Unit,
  modifier: Modifier = Modifier,
) {
  DisposableEffect(pane) { onDispose { onLayoutInfoCleared(pane) } }
  val initialDismissRequestVersion = remember(pane) { dismissRequestVersion }

  AnchoredSheetSurface(
    stops = emptyList(),
    stopPolicy = SheetStop.Policy.KeepAll,
    onDismissed = onDismiss,
    onDismissStarted = onDismissStarted,
    onGeometryChanged = { geometry ->
      onLayoutInfoChanged(
        EditorSubPaneLayoutInfo(
          pane = pane,
          visibleHeight = geometry.visibleHeight,
          visibleAreaMode = EditorSubPaneVisibleAreaMode.ResizeEditor,
        )
      )
    },
    modifier = modifier,
  ) {
    LaunchedEffect(dismissRequestVersion, initialDismissRequestVersion) {
      if (dismissRequestVersion != initialDismissRequestVersion) {
        dismiss()
      }
    }

    PlatformBackHandler(enabled = true) { dismiss() }

    EditorTableAxisActionsSheet(
      target = pane.target,
      currentBackgroundColor = currentBackgroundColor,
      onAction = { message ->
        dismiss()
        onAction(message)
      },
    )
  }
}

@Composable
private fun EditorTableAxisActionsSheet(
  target: EditorTableAxisActionsTarget,
  currentBackgroundColor: Tri<BackgroundColorValue>?,
  onAction: (Message) -> Unit,
) {
  val isRow = target.axis == Axis.Horizontal
  val previousEnabled = target.index > 0
  val nextEnabled = target.index < target.count - 1

  SheetLayout(
    bodyScroll = false,
    includeImeBottomInset = false,
    padding = SheetPadding.None,
    verticalSpacing = 0.dp,
    header = {
      Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        Text(
          text = if (isRow) "행 ${target.index + 1}" else "열 ${target.index + 1}",
          style = AppTheme.typography.title,
          color = AppTheme.colors.textDefault,
          modifier = Modifier.padding(horizontal = TableAxisActionsSheetHorizontalPadding),
        )
        Divider(color = AppTheme.colors.borderDefault)
      }
    },
  ) {
    Column(modifier = Modifier.fillMaxWidth().padding(TableAxisActionsSheetActionPadding)) {
      SheetActionRow(
        icon = if (isRow) Lucide.ArrowUpToLine else Lucide.ArrowLeftToLine,
        label = if (isRow) "위에 행 추가" else "왼쪽에 열 추가",
        contentPadding = TableAxisActionsSheetRowPadding,
        onClick = {
          onAction(
            target.tableMessage(
              TableOp.InsertAxis(axis = target.axis, index = target.index, before = true)
            )
          )
        },
      )
      SheetActionRow(
        icon = if (isRow) Lucide.ArrowDownToLine else Lucide.ArrowRightToLine,
        label = if (isRow) "아래에 행 추가" else "오른쪽에 열 추가",
        contentPadding = TableAxisActionsSheetRowPadding,
        onClick = {
          onAction(
            target.tableMessage(
              TableOp.InsertAxis(axis = target.axis, index = target.index, before = false)
            )
          )
        },
      )
      if (previousEnabled) {
        SheetActionRow(
          icon = if (isRow) Lucide.MoveUp else Lucide.MoveLeft,
          label = if (isRow) "위로 이동" else "왼쪽으로 이동",
          contentPadding = TableAxisActionsSheetRowPadding,
          onClick = {
            onAction(
              target.tableMessage(
                TableOp.MoveAxis(axis = target.axis, from = target.index, to = target.index - 1)
              )
            )
          },
        )
      }
      if (nextEnabled) {
        SheetActionRow(
          icon = if (isRow) Lucide.MoveDown else Lucide.MoveRight,
          label = if (isRow) "아래로 이동" else "오른쪽으로 이동",
          contentPadding = TableAxisActionsSheetRowPadding,
          onClick = {
            onAction(
              target.tableMessage(
                TableOp.MoveAxis(axis = target.axis, from = target.index, to = target.index + 1)
              )
            )
          },
        )
      }
    }

    Divider()
    EditorTableAxisBackgroundColorSection(
      target = target,
      currentBackgroundColor = currentBackgroundColor,
      onSelect = { value ->
        onAction(
          target.tableMessage(
            TableOp.SetAxisBackgroundColor(
              axis = target.axis,
              index = target.index,
              color = value.takeUnless { it == "none" },
            )
          )
        )
      },
    )

    Divider()
    Column(modifier = Modifier.fillMaxWidth().padding(TableAxisActionsSheetActionPadding)) {
      SheetActionRow(
        icon = Lucide.Trash2,
        label =
          when {
            target.count <= 1 -> "테이블 삭제"
            isRow -> "행 삭제"
            else -> "열 삭제"
          },
        contentPadding = TableAxisActionsSheetRowPadding,
        tint = AppTheme.colors.danger,
        onClick = {
          val message =
            if (target.count <= 1) {
              Message.Node(NodeOp.Delete(id = target.tableId))
            } else {
              target.tableMessage(TableOp.DeleteAxis(axis = target.axis, index = target.index))
            }
          onAction(message)
        },
      )
    }
  }
}

@Composable
private fun EditorTableAxisBackgroundColorSection(
  target: EditorTableAxisActionsTarget,
  currentBackgroundColor: Tri<BackgroundColorValue>?,
  onSelect: suspend (String) -> Unit,
) {
  val variant = currentEditorThemeVariant()
  val editorTheme = remember(variant) { EditorTheme.resolve(variant) }

  Column(
    modifier = Modifier.fillMaxWidth().padding(vertical = 14.dp),
    verticalArrangement = Arrangement.spacedBy(10.dp),
  ) {
    Text(
      text = "배경색",
      style = AppTheme.typography.caption,
      color = AppTheme.colors.textMuted,
      modifier = Modifier.padding(horizontal = TableAxisActionsSheetHorizontalPadding),
    )
    EditorSettingsSwatchRow(
      options = EditorValues.textBackgroundColor,
      selected = currentBackgroundColor.currentValue() ?: "",
      onSelect = onSelect,
      theme = editorTheme,
      cornerRadius = AppShapes.sm * 2,
    )
  }
}

private fun Tri<BackgroundColorValue>?.currentValue(): String? =
  when (this) {
    is Tri.Uniform -> value.value
    Tri.Absent -> "none"
    Tri.Mixed,
    null -> null
  }

private fun EditorTableAxisActionsTarget.tableMessage(op: TableOp): Message =
  Message.Node(NodeOp.Table(id = tableId, op = op))
