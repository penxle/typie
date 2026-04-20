package co.typie.ext

internal enum class TextInputKey {
  Enter,
  Backspace,
}

internal interface TextInputClient {
  val hasActiveComposition: Boolean
    get() = false

  fun requestFocus()

  fun insertText(text: String): Boolean = false

  fun commitText(text: String) = Unit

  fun setComposingText(text: String) = Unit

  fun finishComposition() = Unit

  fun pressKey(key: TextInputKey): Boolean = false

  fun dismiss()
}

internal expect fun registerTextInputClient(owner: Any, client: TextInputClient?)
