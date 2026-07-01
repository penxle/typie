package co.typie.screen.editor.editor.toolbar

import co.typie.editor.EditorState
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.Block
import co.typie.editor.ffi.BlockState
import co.typie.editor.ffi.FontSizeValue
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.ModifierState
import co.typie.editor.ffi.PlainNode
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
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
    assertEquals(EditorToolbarTableMode.Selected, context.tableMode)
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
        )
      )

    assertEquals(
      listOf(EditorToolbarPageKey.Main, EditorToolbarPageKey.Text, EditorToolbarPageKey.Table),
      context.pageKeys,
    )
    assertEquals(null, context.autoTargetPageKey)
    assertEquals(EditorToolbarTableMode.InTable, context.tableMode)
  }

  private fun editorState(
    selection: Selection?,
    modifierState: ModifierState? = null,
    blockState: BlockState? = null,
  ): EditorState =
    EditorState(
      version = 1L,
      cursor = null,
      selection = selection,
      pageSizes = emptyList(),
      externalElements = emptyList(),
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
  ): BlockState = BlockState(ancestors = ancestors, nodes = nodes)

  private fun block(id: String, node: PlainNode): Block = Block(id = id, node = node)

  private fun rootNode(): PlainNode.Root = PlainNode.Root(LayoutMode.Continuous(maxWidth = 640))

  private fun modifierState(
    inlineText: Boolean = false,
    alignmentOnly: Boolean = false,
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
    )
}
