package co.typie.screen.editor.editor.toolbar.bottom

import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.InputModifiers
import co.typie.editor.ffi.Key
import co.typie.editor.ffi.KeyEvent
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.BlockquoteVariantPanelTarget
import co.typie.screen.editor.editor.toolbar.EditorToolbarBottomPanel
import co.typie.screen.editor.editor.toolbar.HorizontalRuleVariantPanelTarget
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertSame
import kotlin.test.assertTrue

class NodesTest {
  @Test
  fun insertPanelShowsLineBreakActionOutsideUnitSelection() {
    val item =
      editorToolbarNodeInsertItems(showPageBreak = false, hasUnitSelection = false).single {
        it.label == "문단 내 줄바꿈"
      }

    val action = assertIs<EditorToolbarNodeInsertAction.SendMessage>(item.action)
    assertEquals(Message.Key(KeyEvent(Key.Enter, InputModifiers(shift = true))), action.message)
  }

  @Test
  fun insertPanelShowsInsertAboveActionForUnitSelection() {
    val item =
      editorToolbarNodeInsertItems(showPageBreak = false, hasUnitSelection = true).single {
        it.label == "위에 문단 넣기"
      }

    val action = assertIs<EditorToolbarNodeInsertAction.SendMessage>(item.action)
    assertEquals(Message.Key(KeyEvent(Key.Enter, InputModifiers(shift = true))), action.message)
    assertSame(Lucide.CornerLeftUp, item.icon)
  }

  @Test
  fun horizontalRuleInsertOpensInsertionVariantPanel() {
    val item =
      editorToolbarNodeInsertItems(showPageBreak = false, hasUnitSelection = false).single {
        it.label == "구분선"
      }

    val action = assertIs<EditorToolbarNodeInsertAction.OpenPanel>(item.action)
    assertEquals(
      EditorToolbarBottomPanel.HorizontalRuleVariants(
        target = HorizontalRuleVariantPanelTarget.Insertion
      ),
      action.panel,
    )
  }

  @Test
  fun blockquoteActionOpensSelectionVariantPanel() {
    val item =
      editorToolbarNodeInsertItems(showPageBreak = false, hasUnitSelection = false).single {
        it.label == "인용구"
      }

    val action = assertIs<EditorToolbarNodeInsertAction.OpenPanel>(item.action)
    assertEquals(
      EditorToolbarBottomPanel.BlockquoteVariants(target = BlockquoteVariantPanelTarget.Selection),
      action.panel,
    )
  }

  @Test
  fun tableInsertOpensTableSizePanel() {
    val item =
      editorToolbarNodeInsertItems(showPageBreak = false, hasUnitSelection = false).single {
        it.label == "표"
      }

    val action = assertIs<EditorToolbarNodeInsertAction.OpenPanel>(item.action)
    assertEquals(EditorToolbarBottomPanel.TableSizeSelector, action.panel)
  }

  @Test
  fun singleCharacterTextSelectionDoesNotUseUnitSelectionAction() {
    assertFalse(
      isEditorToolbarUnitSelection(
        selection =
          Selection(
            anchor = Position(node = "text", offset = 1, affinity = Affinity.Downstream),
            head = Position(node = "text", offset = 2, affinity = Affinity.Downstream),
          ),
        hasSelectedBlock = false,
      )
    )
  }

  @Test
  fun singleSlotRangeWithSelectedBlockUsesUnitSelectionAction() {
    assertTrue(
      isEditorToolbarUnitSelection(
        selection =
          Selection(
            anchor = Position(node = "root", offset = 1, affinity = Affinity.Downstream),
            head = Position(node = "root", offset = 2, affinity = Affinity.Downstream),
          ),
        hasSelectedBlock = true,
      )
    )
  }
}
