package co.typie.screen.editor.editor.toolbar

import co.typie.editor.EditorState
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Alignment
import co.typie.editor.ffi.BackgroundColorValue
import co.typie.editor.ffi.Block
import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.FontSizeValue
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.ListAffordances
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect as FfiRect
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.TableBorderStyle
import co.typie.editor.ffi.TableOverlay
import co.typie.editor.ffi.Tri
import kotlin.test.Test
import kotlin.test.assertEquals

class ToolbarContextTest {
  @Test
  fun collapsedPlainParagraphShowsTextWithoutAutoTarget() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          modifierState = modifierState(inlineText = true),
          blockState =
            blockState(
              ancestors = listOf(block("paragraph", PlainNode.Paragraph), block("root", rootNode()))
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text), context.pageKeys)
    assertEquals(null, context.autoTargetPageKey)
  }

  @Test
  fun nonCollapsedTextSelectionShowsTextWithoutAutoTarget() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = rangeSelection(),
          modifierState =
            modifierState(
              inlineText = true,
              cellBackgroundColor = Tri.Uniform(BackgroundColorValue("yellow")),
            ),
          blockState =
            blockState(
              ancestors = listOf(block("paragraph", PlainNode.Paragraph), block("root", rootNode()))
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text), context.pageKeys)
    assertEquals(null, context.autoTargetPageKey)
  }

  @Test
  fun selectedImageAutoTargetsImageWithoutTextPage() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = singleBlockSelection(),
          modifierState = modifierState(alignmentOnly = true),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              nodes = listOf(block("image", PlainNode.Image(id = null))),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Image), context.pageKeys)
    assertEquals(EditorToolbarPageKey.Image, context.autoTargetPageKey)
    assertEquals(
      EditorToolbarAutoTargetKey(pageKey = EditorToolbarPageKey.Image, selectedNodeId = "image"),
      context.autoTargetKey,
    )
    assertEquals("image", context.selectedNodeId)
  }

  @Test
  fun selectedTableAutoTargetsTableInSelectedMode() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = singleBlockSelection(),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              nodes = listOf(block("table", PlainNode.Table())),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Table), context.pageKeys)
    assertEquals(EditorToolbarPageKey.Table, context.autoTargetPageKey)
    assertEquals(
      EditorToolbarAutoTargetKey(pageKey = EditorToolbarPageKey.Table, selectedNodeId = "table"),
      context.autoTargetKey,
    )
    assertEquals(
      EditorToolbarTableTarget(
        id = "table",
        node = PlainNode.Table(),
        selected = true,
        align = Alignment.Left,
        cellBackgroundCurrentValue = null,
      ),
      context.tableTarget,
    )
  }

  @Test
  fun selectedBlockquoteAutoTargetsBlockquote() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = singleBlockSelection(),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              nodes = listOf(block("blockquote", PlainNode.Blockquote())),
            ),
        )
      )

    assertEquals(
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Blockquote),
      context.pageKeys,
    )
    assertEquals(EditorToolbarPageKey.Blockquote, context.autoTargetPageKey)
    assertEquals(
      EditorToolbarAutoTargetKey(
        pageKey = EditorToolbarPageKey.Blockquote,
        selectedNodeId = "blockquote",
      ),
      context.autoTargetKey,
    )
    assertEquals(
      EditorToolbarNodeTarget(id = "blockquote", node = PlainNode.Blockquote()),
      context.blockquoteTarget,
    )
  }

  @Test
  fun selectedCalloutAutoTargetsCallout() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = singleBlockSelection(),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              nodes = listOf(block("callout", PlainNode.Callout())),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Callout), context.pageKeys)
    assertEquals(EditorToolbarPageKey.Callout, context.autoTargetPageKey)
    assertEquals(
      EditorToolbarAutoTargetKey(
        pageKey = EditorToolbarPageKey.Callout,
        selectedNodeId = "callout",
      ),
      context.autoTargetKey,
    )
    assertEquals(
      EditorToolbarNodeTarget(id = "callout", node = PlainNode.Callout()),
      context.calloutTarget,
    )
  }

  @Test
  fun selectedFoldAutoTargetsFold() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = singleBlockSelection(),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              nodes = listOf(block("fold", PlainNode.Fold)),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Fold), context.pageKeys)
    assertEquals(EditorToolbarPageKey.Fold, context.autoTargetPageKey)
    assertEquals(
      EditorToolbarAutoTargetKey(pageKey = EditorToolbarPageKey.Fold, selectedNodeId = "fold"),
      context.autoTargetKey,
    )
    assertEquals("fold", context.foldTargetId)
  }

  @Test
  fun ancestorContextsAreIncludedAfterText() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = rangeSelection(),
          modifierState = modifierState(inlineText = true),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("list-item", PlainNode.ListItem),
                  block("bullet-list", PlainNode.BulletList),
                  block("blockquote", PlainNode.Blockquote()),
                  block("root", rootNode()),
                )
            ),
        )
      )

    assertEquals(
      listOf(
        EditorToolbarPageKey.Main,
        EditorToolbarPageKey.Text,
        EditorToolbarPageKey.List,
        EditorToolbarPageKey.Blockquote,
      ),
      context.pageKeys,
    )
    assertEquals(null, context.autoTargetPageKey)
  }

  @Test
  fun collapsedCursorInsideListShowsListMode() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("list-item", PlainNode.ListItem),
                  block("bullet-list", PlainNode.BulletList),
                  block("root", rootNode()),
                )
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.List), context.pageKeys)
    assertEquals(EditorToolbarListMode.Bullet, context.listMode)
  }

  @Test
  fun collapsedCursorInsideNestedListUsesClosestListMode() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("child-list-item", PlainNode.ListItem),
                  block("child-bullet-list", PlainNode.BulletList),
                  block("parent-list-item", PlainNode.ListItem),
                  block("parent-ordered-list", PlainNode.OrderedList),
                  block("root", rootNode()),
                )
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.List), context.pageKeys)
    assertEquals(EditorToolbarListMode.Bullet, context.listMode)
  }

  @Test
  fun nonCollapsedRangeInsideNestedListUsesClosestListMode() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = rangeSelection(),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("child-list-item", PlainNode.ListItem),
                  block("child-bullet-list", PlainNode.BulletList),
                  block("parent-list-item", PlainNode.ListItem),
                  block("parent-ordered-list", PlainNode.OrderedList),
                  block("root", rootNode()),
                ),
              intersectingNodes =
                listOf(
                  block("parent-ordered-list", PlainNode.OrderedList),
                  block("parent-list-item", PlainNode.ListItem),
                  block("child-bullet-list", PlainNode.BulletList),
                  block("child-list-item", PlainNode.ListItem),
                  block("paragraph", PlainNode.Paragraph),
                ),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.List), context.pageKeys)
    assertEquals(EditorToolbarListMode.Bullet, context.listMode)
  }

  @Test
  fun nonCollapsedRangeAcrossNestedMixedListsShowsNoSelectedMode() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = rangeSelection(),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("parent-list-item", PlainNode.ListItem),
                  block("parent-ordered-list", PlainNode.OrderedList),
                  block("root", rootNode()),
                ),
              intersectingNodes =
                listOf(
                  block("parent-ordered-list", PlainNode.OrderedList),
                  block("parent-list-item", PlainNode.ListItem),
                  block("parent-paragraph", PlainNode.Paragraph),
                  block("child-bullet-list", PlainNode.BulletList),
                  block("child-list-item", PlainNode.ListItem),
                  block("child-paragraph", PlainNode.Paragraph),
                ),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.List), context.pageKeys)
    assertEquals(null, context.listMode)
  }

  @Test
  fun nonCollapsedRangeContainingListShowsListWithoutAutoTarget() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = rangeSelection(),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              intersectingNodes =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("list-item", PlainNode.ListItem),
                ),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.List), context.pageKeys)
    assertEquals(null, context.autoTargetPageKey)
    assertEquals(null, context.listMode)
  }

  @Test
  fun nonCollapsedUnsupportedMixedRangeContainingListStillShowsList() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = rangeSelection(),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              nodes = listOf(block("image", PlainNode.Image(id = null))),
              intersectingNodes =
                listOf(
                  block("image", PlainNode.Image(id = null)),
                  block("list-item", PlainNode.ListItem),
                ),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.List), context.pageKeys)
    assertEquals(null, context.autoTargetPageKey)
  }

  @Test
  fun nonCollapsedMixedListKindsShowListWithoutSelectedMode() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = rangeSelection(),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              nodes =
                listOf(
                  block("bullet-list", PlainNode.BulletList),
                  block("ordered-list", PlainNode.OrderedList),
                ),
              intersectingNodes =
                listOf(
                  block("bullet-list", PlainNode.BulletList),
                  block("ordered-list", PlainNode.OrderedList),
                ),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.List), context.pageKeys)
    assertEquals(null, context.listMode)
  }

  @Test
  fun cursorInsideTableAddsTableInModeWithoutAutoTarget() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          modifierState = modifierState(inlineText = true),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("cell", PlainNode.TableCell(colWidth = null, backgroundColor = null)),
                  block("row", PlainNode.TableRow),
                  block("table", PlainNode.Table()),
                  block("root", rootNode()),
                )
            ),
          tableOverlays = listOf(tableOverlay(tableId = "table", align = Alignment.Right)),
        )
      )

    assertEquals(
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Table),
      context.pageKeys,
    )
    assertEquals(null, context.autoTargetPageKey)
    assertEquals(
      EditorToolbarTableTarget(
        id = "table",
        node = PlainNode.Table(),
        selected = false,
        align = Alignment.Right,
        cellBackgroundCurrentValue = null,
      ),
      context.tableTarget,
    )
  }

  @Test
  fun cursorInsideColoredTableCellUsesModifierStateCellBackground() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          modifierState =
            modifierState(
              inlineText = true,
              cellBackgroundColor = Tri.Uniform(BackgroundColorValue("yellow")),
            ),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("cell", PlainNode.TableCell(colWidth = null, backgroundColor = null)),
                  block("row", PlainNode.TableRow),
                  block("table", PlainNode.Table()),
                  block("root", rootNode()),
                )
            ),
          tableOverlays = listOf(tableOverlay(tableId = "table", align = Alignment.Right)),
        )
      )

    assertEquals("yellow", context.tableTarget?.cellBackgroundCurrentValue)
  }

  @Test
  fun tableCellBackgroundAbsentActivatesNoneSwatch() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          modifierState = modifierState(inlineText = true, cellBackgroundColor = Tri.Absent),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("cell", PlainNode.TableCell(colWidth = null, backgroundColor = null)),
                  block("row", PlainNode.TableRow),
                  block("table", PlainNode.Table()),
                  block("root", rootNode()),
                )
            ),
          tableOverlays = listOf(tableOverlay(tableId = "table", align = Alignment.Right)),
        )
      )

    assertEquals("none", context.tableTarget?.cellBackgroundCurrentValue)
  }

  @Test
  fun mixedTableCellBackgroundLeavesNoActiveSwatch() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          modifierState = modifierState(inlineText = true, cellBackgroundColor = Tri.Mixed),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("cell", PlainNode.TableCell(colWidth = null, backgroundColor = null)),
                  block("row", PlainNode.TableRow),
                  block("table", PlainNode.Table()),
                  block("root", rootNode()),
                )
            ),
          tableOverlays = listOf(tableOverlay(tableId = "table", align = Alignment.Right)),
        )
      )

    assertEquals(null, context.tableTarget?.cellBackgroundCurrentValue)
  }

  @Test
  fun rangeOutsideTableDoesNotCreateAmbiguousTableTarget() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = rangeSelection(),
          blockState =
            blockState(
              ancestors = listOf(block("root", rootNode())),
              intersectingNodes =
                listOf(block("paragraph", PlainNode.Paragraph), block("table", PlainNode.Table())),
            ),
        )
      )

    assertEquals(listOf(EditorToolbarPageKey.Main), context.pageKeys)
    assertEquals(null, context.tableTarget)
  }

  @Test
  fun cursorInsideFoldTitleKeepsFoldPageAvailableWithoutAutoTarget() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          modifierState = modifierState(inlineText = true),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("fold-title", PlainNode.FoldTitle),
                  block("fold", PlainNode.Fold),
                  block("root", rootNode()),
                )
            ),
        )
      )

    assertEquals(
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold),
      context.pageKeys,
    )
    assertEquals(null, context.autoTargetPageKey)
    assertEquals(null, context.autoTargetKey)
    assertEquals("fold", context.foldTargetId)
  }

  @Test
  fun cursorInsideFoldContentKeepsFoldPageAvailableWithoutAutoTarget() {
    val context =
      resolveEditorToolbarContext(
        editorState(
          selection = collapsedSelection(),
          modifierState = modifierState(inlineText = true),
          blockState =
            blockState(
              ancestors =
                listOf(
                  block("paragraph", PlainNode.Paragraph),
                  block("fold-content", PlainNode.FoldContent),
                  block("fold", PlainNode.Fold),
                  block("root", rootNode()),
                )
            ),
        )
      )

    assertEquals(
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Fold),
      context.pageKeys,
    )
    assertEquals(null, context.autoTargetPageKey)
    assertEquals(null, context.autoTargetKey)
    assertEquals("fold", context.foldTargetId)
  }

  private fun editorState(
    selection: Selection?,
    modifierState: ModifierState? = null,
    blockState: BlockState? = null,
    tableOverlays: List<TableOverlay> = emptyList(),
  ): EditorState =
    EditorState(
      version = 1L,
      cursor = null,
      selection = selection,
      pageSizes = emptyList(),
      externalElements = emptyList(),
      tableOverlays = tableOverlays,
      rootAttrs = null,
      rootModifiers = null,
      modifierState = modifierState,
      blockState = blockState,
      ime = null,
    )

  private fun collapsedSelection(): Selection =
    Selection(
      anchor = Position(node = "text", offset = 0, affinity = Affinity.Downstream),
      head = Position(node = "text", offset = 0, affinity = Affinity.Downstream),
    )

  private fun rangeSelection(): Selection =
    Selection(
      anchor = Position(node = "text", offset = 0, affinity = Affinity.Downstream),
      head = Position(node = "text", offset = 2, affinity = Affinity.Downstream),
    )

  private fun singleBlockSelection(): Selection =
    Selection(
      anchor = Position(node = "root", offset = 1, affinity = Affinity.Downstream),
      head = Position(node = "root", offset = 2, affinity = Affinity.Downstream),
    )

  private fun blockState(
    ancestors: List<Block> = emptyList(),
    nodes: List<Block> = emptyList(),
    intersectingNodes: List<Block> = emptyList(),
  ): BlockState =
    BlockState(
      ancestors = ancestors,
      nodes = nodes,
      intersectingNodes = intersectingNodes,
      list =
        ListAffordances(
          toggleBullet = false,
          toggleOrdered = false,
          indent = false,
          outdent = false,
        ),
    )

  private fun block(id: String, node: PlainNode): Block = Block(id = id, node = node)

  private fun rootNode(): PlainNode.Root = PlainNode.Root(LayoutMode.Continuous(maxWidth = 640))

  private fun tableOverlay(tableId: String, align: Alignment): TableOverlay =
    TableOverlay(
      tableId = tableId,
      pageIdx = 0,
      bounds = FfiRect(x = 0f, y = 0f, width = 100f, height = 80f),
      borderStyle = TableBorderStyle.Solid,
      align = align,
      proportion = 1f,
      contentWidth = 100f,
      minProportionWidth = 80f,
      maxProportionWidth = 160f,
      rows = emptyList(),
      columns = emptyList(),
      rowCount = 0,
      isLastRowFragment = true,
      isFocused = true,
      focusedRowIndex = null,
      focusedColIndex = null,
      cellSelection = null,
    )

  private fun modifierState(
    inlineText: Boolean = false,
    alignmentOnly: Boolean = false,
    cellBackgroundColor: Tri<BackgroundColorValue>? = null,
  ): ModifierState =
    ModifierState(
      bold = Tri.Absent,
      italic = Tri.Absent,
      underline = Tri.Absent,
      strikethrough = Tri.Absent,
      fontSize = if (inlineText) Tri.Uniform(FontSizeValue(1200)) else Tri.Absent,
      fontFamily = Tri.Absent,
      fontWeight = Tri.Absent,
      textColor = Tri.Absent,
      backgroundColor = Tri.Absent,
      letterSpacing = Tri.Absent,
      link = Tri.Absent,
      ruby = Tri.Absent,
      lineHeight = Tri.Absent,
      blockGap = Tri.Absent,
      paragraphIndent = Tri.Absent,
      alignment = if (alignmentOnly) Tri.Mixed else Tri.Absent,
      effectiveBold = Tri.Absent,
      cellBackgroundColor = cellBackgroundColor,
    )
}
