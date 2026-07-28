package co.typie.editor

import co.typie.editor.ffi.Message

interface EditorRequestScope {
  fun enqueue(message: Message)
}
