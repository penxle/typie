package co.typie.editor

import co.typie.editor.ffi.Message

interface EditorScope {
  fun enqueue(message: Message)
}
