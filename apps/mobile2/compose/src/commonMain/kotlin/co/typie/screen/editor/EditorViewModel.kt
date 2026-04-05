package co.typie.screen.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.editor.Editor
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.DocumentAttrs
import co.typie.editor.ffi.LayoutMode
import co.typie.editor.ffi.Modifier
import co.typie.editor.ffi.Node
import co.typie.editor.ffi.NodeEntry
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Viewport
import kotlinx.coroutines.launch
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class EditorViewModel : ViewModel() {
  var editor by mutableStateOf<Editor?>(null)
    private set

  fun initialize(scaleFactor: Double) {
    if (editor != null) return

    val doc = Doc(
      nodes = mapOf(
        "0" to NodeEntry(
          node = Node.Root,
          modifiers = listOf(
            Modifier.FontFamily("Pretendard"),
            Modifier.FontWeight(400),
            Modifier.FontSize(1200),
            Modifier.LineHeight(160),
            Modifier.LetterSpacing(0),
            Modifier.TextColor("black"),
            Modifier.ParagraphIndent(100),
            Modifier.BlockGap(100),
          ),
          children = listOf("10"),
        ),
        "10" to NodeEntry(
          node = Node.Blockquote(),
          parent = "0",
          children = listOf("1", "3", "5"),
        ),
        "1" to NodeEntry(
          node = Node.Paragraph(),
          parent = "10",
          children = listOf("2"),
        ),
        "2" to NodeEntry(
          node = Node.Text("A"),
          parent = "1",
        ),
        "3" to NodeEntry(
          node = Node.Paragraph(),
          parent = "10",
          children = listOf("4"),
        ),
        "4" to NodeEntry(
          node = Node.Text("Hello, World!"),
          parent = "3",
        ),
        "5" to NodeEntry(
          node = Node.Paragraph(),
          parent = "10",
          children = listOf("6"),
        ),
        "6" to NodeEntry(
          node = Node.Text("안녕하세요!"),
          parent = "5",
        ),
      ),
      attrs = DocumentAttrs(layoutMode = LayoutMode.Continuous(maxWidth = 400f)),
    )

    val selection = Selection(
      anchor = Position("4", 0),
      head = Position("4", 0),
    )

    val viewport = Viewport(
      width = 600f,
      height = 400f,
      scaleFactor = scaleFactor,
    )

    viewModelScope.launch {
      editor = Editor.create(doc, selection, viewport, viewModelScope)
    }
  }
}
