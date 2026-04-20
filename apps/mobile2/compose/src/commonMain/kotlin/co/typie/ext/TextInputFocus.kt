package co.typie.ext

import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusEventModifierNode
import androidx.compose.ui.focus.FocusState
import androidx.compose.ui.node.ModifierNodeElement

internal expect fun notifyTextInputFocusChanged(owner: Any, isFocused: Boolean)

internal fun Modifier.textInputFocusChanged(
  owner: Any? = null,
  enabled: Boolean = true,
  onFocusChange: (FocusState) -> Unit = {},
): Modifier = this then TextInputFocusElement(owner, enabled, onFocusChange)

private data class TextInputFocusElement(
  private val owner: Any?,
  private val enabled: Boolean,
  private val onFocusChange: (FocusState) -> Unit,
) : ModifierNodeElement<TextInputFocusNode>() {
  override fun create(): TextInputFocusNode =
    TextInputFocusNode(owner = owner, enabled = enabled, onFocusChange = onFocusChange)

  override fun update(node: TextInputFocusNode) {
    node.update(owner = owner, enabled = enabled, onFocusChange = onFocusChange)
  }
}

private class TextInputFocusNode(
  private var owner: Any?,
  private var enabled: Boolean,
  private var onFocusChange: (FocusState) -> Unit,
) : Modifier.Node(), FocusEventModifierNode {
  override fun onFocusEvent(focusState: FocusState) {
    notifyTextInputFocusChanged(owner ?: this, enabled && focusState.isFocused)
    onFocusChange(focusState)
  }

  fun update(owner: Any?, enabled: Boolean, onFocusChange: (FocusState) -> Unit) {
    this.owner = owner
    this.onFocusChange = onFocusChange
    if (this.enabled == enabled) return

    this.enabled = enabled
    if (!enabled) {
      notifyTextInputFocusChanged(owner ?: this, false)
    }
  }

  override fun onDetach() {
    notifyTextInputFocusChanged(owner ?: this, false)
    super.onDetach()
  }
}
