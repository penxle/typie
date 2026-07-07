package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import co.typie.editor.ffi.ListKind
import co.typie.editor.ffi.ListOp
import co.typie.editor.ffi.Message
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarDivider
import co.typie.screen.editor.editor.toolbar.EditorToolbarListMode
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow

internal fun editorListToolbarPage(mode: EditorToolbarListMode?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.List,
    icon = Lucide.List,
    contentDescription = "목록 툴바",
    content = { scope ->
      val editor = LocalEditorRuntime.current.editor
      val editorVersion = editor?.state?.version
      val canSetBullet by
        produceState(initialValue = false, editor, editorVersion) {
          value = false
          value = editor?.can(Message.List(ListOp.SetKind(ListKind.Bullet))) == true
        }
      val canSetOrdered by
        produceState(initialValue = false, editor, editorVersion) {
          value = false
          value = editor?.can(Message.List(ListOp.SetKind(ListKind.Ordered))) == true
        }
      val canIndent by
        produceState(initialValue = false, editor, editorVersion) {
          value = false
          value = editor?.can(Message.List(ListOp.Indent)) == true
        }
      val canOutdent by
        produceState(initialValue = false, editor, editorVersion) {
          value = false
          value = editor?.can(Message.List(ListOp.Outdent)) == true
        }
      EditorToolbarRow(scope = scope) {
        EditorToolbarButton(
          icon = Lucide.Dot,
          contentDescription = "글머리 목록",
          selected = mode == EditorToolbarListMode.Bullet,
          enabled = mode == EditorToolbarListMode.Bullet || canSetBullet,
          onClick = { scope.sendMessage(Message.List(ListOp.SetKind(ListKind.Bullet))) },
        )
        EditorToolbarButton(
          icon = Lucide.Hash,
          contentDescription = "번호 목록",
          selected = mode == EditorToolbarListMode.Ordered,
          enabled = mode == EditorToolbarListMode.Ordered || canSetOrdered,
          onClick = { scope.sendMessage(Message.List(ListOp.SetKind(ListKind.Ordered))) },
        )
        EditorToolbarDivider()
        EditorToolbarButton(
          icon = Lucide.IndentIncrease,
          contentDescription = "들여쓰기",
          enabled = canIndent,
          onClick = { scope.sendMessage(Message.List(ListOp.Indent)) },
        )
        EditorToolbarButton(
          icon = Lucide.IndentDecrease,
          contentDescription = "내어쓰기",
          enabled = canOutdent,
          onClick = { scope.sendMessage(Message.List(ListOp.Outdent)) },
        )
      }
    },
  )
