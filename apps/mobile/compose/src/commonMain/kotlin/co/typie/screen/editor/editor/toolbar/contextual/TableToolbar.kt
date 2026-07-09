package co.typie.screen.editor.editor.toolbar.contextual

import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.TableOp
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarBottomPanel
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow
import co.typie.screen.editor.editor.toolbar.EditorToolbarSecondary
import co.typie.screen.editor.editor.toolbar.EditorToolbarTableTarget
import co.typie.screen.editor.editor.toolbar.TableBorderStylePanelTarget

internal fun editorTableToolbarPage(target: EditorToolbarTableTarget?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Table,
    icon = Lucide.Table,
    contentDescription = "표 툴바",
    ownerNodeId = target?.id,
    content = { scope ->
      EditorToolbarRow(scope = scope) {
        val currentTarget = target ?: return@EditorToolbarRow
        val alignmentSecondary = EditorToolbarSecondary.TableAlignment(currentTarget.id)
        val cellBackgroundSecondary = EditorToolbarSecondary.TableCellBackground(currentTarget.id)
        val borderPanel =
          EditorToolbarBottomPanel.TableBorderStyles(
            TableBorderStylePanelTarget(
              tableId = currentTarget.id,
              currentStyle = currentTarget.node.borderStyle,
            )
          )
        EditorToolbarButton(
          icon = Lucide.GripVertical,
          contentDescription = "표 전체 선택",
          selected = currentTarget.selected,
          onClick = {
            if (!currentTarget.selected) {
              scope.sendMessage(tableSelectAllMessage(currentTarget.id))
            }
          },
        )
        EditorToolbarButton(
          icon = toolbarAlignmentIcon(currentTarget.align),
          contentDescription = "표 정렬",
          selected = scope.activeSecondaryToolbar == alignmentSecondary,
          enabled = currentTarget.node.proportion != 100,
          onClick = { scope.toggleSecondaryToolbar(alignmentSecondary) },
        )
        EditorToolbarButton(
          icon = Lucide.TableProperties,
          contentDescription = "표 테두리",
          selected =
            (scope.activeBottomPanel as? EditorToolbarBottomPanel.TableBorderStyles)
              ?.target
              ?.tableId == currentTarget.id,
          onClick = { scope.toggleBottomPanel(borderPanel) },
        )
        EditorToolbarButton(
          icon = Lucide.PaintBucket,
          contentDescription = "셀 배경색",
          selected = scope.activeSecondaryToolbar == cellBackgroundSecondary,
          onClick = { scope.toggleSecondaryToolbar(cellBackgroundSecondary) },
        )
      }
    },
  )

internal fun tableSelectAllMessage(tableId: String): Message =
  Message.Node(NodeOp.Table(id = tableId, op = TableOp.SelectAxis(axis = null, index = null)))
