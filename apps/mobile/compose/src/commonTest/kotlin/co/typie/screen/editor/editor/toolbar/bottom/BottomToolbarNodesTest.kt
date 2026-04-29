package co.typie.screen.editor.editor.toolbar.bottom

import co.typie.editor.ffi.Break
import co.typie.editor.ffi.InsertionOp
import co.typie.editor.ffi.Node
import co.typie.icons.Lucide
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class BottomToolbarNodesTest {
  @Test
  fun nodeInsertItemsIncludePageAndLineBreaks() {
    val items = editorToolbarNodeInsertItems(showPageBreak = true)

    assertEquals(
      listOf("이미지", "파일", "임베드", "구분선", "인용구", "강조", "접기", "표", "목록", "페이지 나누기", "강제 줄바꿈"),
      items.map { it.label },
    )

    assertEquals(InsertionOp.Break(Break.Page), items.single { it.label == "페이지 나누기" }.message.op)
    assertEquals(InsertionOp.Break(Break.Line), items.single { it.label == "강제 줄바꿈" }.message.op)
  }

  @Test
  fun nodeInsertItemsHidePageBreakOutsidePaginatedLayout() {
    val items = editorToolbarNodeInsertItems(showPageBreak = false)

    assertEquals(
      listOf("이미지", "파일", "임베드", "구분선", "인용구", "강조", "접기", "표", "목록", "강제 줄바꿈"),
      items.map { it.label },
    )
  }

  @Test
  fun nodeInsertItemsUseWebFfiToolbarIcons() {
    val items = editorToolbarNodeInsertItems(showPageBreak = true)

    assertEquals(Lucide.Scissors, items.single { it.label == "구분선" }.icon)
    assertEquals(Lucide.FilePlus, items.single { it.label == "페이지 나누기" }.icon)
  }

  @Test
  fun nodeInsertItemsUseEditorFfiFragmentsForBlockNodes() {
    val items = editorToolbarNodeInsertItems(showPageBreak = true)

    val horizontalRule =
      assertIs<InsertionOp.Fragment>(items.single { it.label == "구분선" }.message.op)
    val blockquote = assertIs<InsertionOp.Fragment>(items.single { it.label == "인용구" }.message.op)
    val table = assertIs<InsertionOp.Fragment>(items.single { it.label == "표" }.message.op)

    assertEquals(Node.HorizontalRule(), horizontalRule.fragment.node)
    assertEquals(Node.Blockquote(), blockquote.fragment.node)
    assertEquals(Node.Table(), table.fragment.node)
  }
}
