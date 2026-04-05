package co.typie.editor.compose

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInParent
import androidx.compose.ui.platform.LocalDensity
import co.typie.editor.Editor
import co.typie.ext.verticalScroll
import co.typie.ui.state.rememberScrollState
import androidx.compose.ui.geometry.Size as ComposeSize

@Composable
fun EditorView(editor: Editor) {
  val density = LocalDensity.current
  val scrollState = rememberScrollState()
  val focusRequester = remember { FocusRequester() }
  val pageSizes = editor.pageSizes
  val pageOffsets = remember { mutableStateMapOf<Int, Offset>() }

  Box(Modifier.fillMaxSize()) {
    Box(
      Modifier.fillMaxWidth()
        .focusRequester(focusRequester)
        .editorTextInput(editor)
        .focusable()
        .verticalScroll(scrollState)
        .editorGestures(editor, focusRequester, pageOffsets, pageSizes)
    ) {
      Column {
        pageSizes.forEachIndexed { index, size ->
          Page(
            editor = editor,
            page = index,
            width = size.width,
            height = size.height,
            modifier = Modifier.onGloballyPositioned { coordinates ->
              val pos = coordinates.positionInParent()
              pageOffsets[index] = with(density) {
                Offset(pos.x.toDp().value, pos.y.toDp().value)
              }
            },
          )
        }
      }

      val cursor = editor.cursor
      if (cursor != null) {
        val cursorGlobal = localToGlobal(
          page = cursor.pageIdx.toInt(),
          x = cursor.rect.x,
          y = cursor.rect.y,
          pageOffsets = pageOffsets,
        )

        if (cursorGlobal != null) {
          Cursor(
            offset = cursorGlobal,
            size = ComposeSize(cursor.rect.width, cursor.rect.height),
          )
        }
      }
    }
  }
}
