@file:OptIn(kotlinx.cinterop.ExperimentalForeignApi::class)

package co.typie.ui.utils

import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusEventModifierNode
import androidx.compose.ui.focus.FocusState
import androidx.compose.ui.node.CompositionLocalConsumerModifierNode
import androidx.compose.ui.node.ModifierNodeElement
import androidx.compose.ui.node.currentValueOf
import platform.UIKit.UIKeyModifierAlternate
import platform.UIKit.UIKeyModifierCommand
import platform.UIKit.UIKeyModifierControl
import platform.UIKit.UIKeyModifierShift

internal fun Modifier.nativeShortcut(
  input: String,
  modifierFlags: Long,
  enabled: Boolean,
  onShortcut: () -> Unit,
): Modifier = this then NativeShortcutElement(input, modifierFlags, enabled, onShortcut)

internal fun nativeShortcutModifierFlags(modifiers: Set<ShortcutModifier>): Long {
  var flags = 0L
  if (ShortcutModifier.Shift in modifiers) flags = flags or UIKeyModifierShift
  if (ShortcutModifier.Ctrl in modifiers) flags = flags or UIKeyModifierControl
  if (ShortcutModifier.Mod in modifiers) flags = flags or UIKeyModifierCommand
  if (ShortcutModifier.Alt in modifiers) flags = flags or UIKeyModifierAlternate
  return flags
}

private data class NativeShortcutElement(
  private val input: String,
  private val modifierFlags: Long,
  private val enabled: Boolean,
  private val onShortcut: () -> Unit,
) : ModifierNodeElement<NativeShortcutNode>() {
  override fun create(): NativeShortcutNode =
    NativeShortcutNode(input, modifierFlags, enabled, onShortcut)

  override fun update(node: NativeShortcutNode) {
    node.update(input, modifierFlags, enabled, onShortcut)
  }
}

internal class NativeShortcutNode(
  var input: String,
  var modifierFlags: Long,
  private var enabled: Boolean,
  var onShortcut: () -> Unit,
) : Modifier.Node(), FocusEventModifierNode, CompositionLocalConsumerModifierNode {
  private var focused = false
  private var registry: NativeShortcutRegistry? = null

  override fun onAttach() {
    registry = currentValueOf(LocalNativeShortcutRegistry)
    syncRegistration()
  }

  override fun onFocusEvent(focusState: FocusState) {
    focused = focusState.isFocused
    syncRegistration()
  }

  override fun onDetach() {
    registry?.deactivate(this)
    registry = null
    super.onDetach()
  }

  fun update(input: String, modifierFlags: Long, enabled: Boolean, onShortcut: () -> Unit) {
    this.input = input
    this.modifierFlags = modifierFlags
    this.enabled = enabled
    this.onShortcut = onShortcut
    syncRegistration()
  }

  private fun syncRegistration() {
    val registry = registry ?: return
    if (isAttached && focused && enabled) {
      registry.activate(this)
    } else {
      registry.deactivate(this)
    }
  }
}

class NativeShortcutRegistry {
  private var active: NativeShortcutNode? = null

  internal fun activate(node: NativeShortcutNode) {
    active = node
  }

  internal fun deactivate(node: NativeShortcutNode) {
    if (active === node) {
      active = null
    }
  }

  fun activeInput(): String? = active?.input

  fun activeModifierFlags(): Long = active?.modifierFlags ?: 0L

  fun dispatch(input: String, modifierFlags: Long) {
    active?.takeIf { it.input == input && it.modifierFlags == modifierFlags }?.onShortcut?.invoke()
  }
}

internal val LocalNativeShortcutRegistry =
  staticCompositionLocalOf<NativeShortcutRegistry> { error("No NativeShortcutRegistry provided") }
