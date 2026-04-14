package co.typie.editor.compose

import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size as ComposeSize
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.layout.positionInParent
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import co.typie.editor.Editor
import co.typie.editor.LocalEditorState
import co.typie.editor.ffi.Doc
import co.typie.editor.ffi.Selection
import co.typie.editor.ffi.Viewport
import co.typie.ext.verticalScroll
import co.typie.platform.PlatformModule
import co.typie.ui.state.rememberScrollState
import kotlinx.coroutines.launch

@Composable
fun EditorView(doc: Doc, selection: Selection) {
  val platform = PlatformModule.platform
  val density = LocalDensity.current
  val scope = rememberCoroutineScope()
  val scrollState = rememberScrollState()

  val ctx = LocalEditorState.current
  var initializing by remember { mutableStateOf(false) }

  Box(
    Modifier.fillMaxSize().onSizeChanged { size ->
      if (!initializing) {
        initializing = true
        scope.launch {
          ctx.editor =
            Editor.create(
              doc,
              selection,
              Viewport(
                width = size.width / density.density,
                height = size.height / density.density,
                scaleFactor = density.density.toDouble(),
              ),
              scope,
            )
        }
      }
    }
  ) {
    val editor = ctx.editor ?: return@Box
    editor.focusManager = LocalFocusManager.current

    Box(
      Modifier.fillMaxWidth()
        .focusRequester(editor.focusRequester)
        .editorInput(editor, platform)
        .focusable()
        .verticalScroll(scrollState)
        .editorGestures(editor)
    ) {
      Column {
        editor.pageSizes.forEachIndexed { index, size ->
          Page(
            editor = editor,
            page = index,
            width = size.width,
            height = size.height,
            modifier =
              Modifier.onGloballyPositioned { coordinates ->
                val pos = coordinates.positionInParent()
                editor.pageOffsets[index] =
                  with(density) { Offset(pos.x.toDp().value, pos.y.toDp().value) }
              },
          )
        }
      }

      val cursor = editor.cursor
      if (cursor != null) {
        val cursorGlobal =
          editor.localToGlobal(page = cursor.pageIdx, x = cursor.rect.x, y = cursor.rect.y)

        if (cursorGlobal != null) {
          Cursor(offset = cursorGlobal, size = ComposeSize(cursor.rect.width, cursor.rect.height))
        }
      }
    }
  }
}
