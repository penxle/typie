package co.typie.ui.utils

import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusEventModifierNode
import androidx.compose.ui.focus.FocusState
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.nativeKeyCode
import androidx.compose.ui.input.key.nativeKeyLocation
import androidx.compose.ui.node.ModifierNodeElement
import java.awt.KeyEventDispatcher
import java.awt.KeyboardFocusManager
import java.awt.event.KeyEvent

internal actual val platformModUsesMeta: Boolean =
  System.getProperty("os.name").startsWith("Mac", ignoreCase = true)

internal actual fun Modifier.onShortcut(
  key: Key,
  modifiers: Set<ShortcutModifier>,
  enabled: Boolean,
  onShortcut: () -> Unit,
): Modifier = this then DesktopShortcutElement(key, modifiers, enabled, onShortcut)

private data class DesktopShortcutElement(
  private val key: Key,
  private val modifiers: Set<ShortcutModifier>,
  private val enabled: Boolean,
  private val onShortcut: () -> Unit,
) : ModifierNodeElement<DesktopShortcutNode>() {
  override fun create(): DesktopShortcutNode =
    DesktopShortcutNode(key, modifiers, enabled, onShortcut)

  override fun update(node: DesktopShortcutNode) {
    node.update(key, modifiers, enabled, onShortcut)
  }
}

private class DesktopShortcutNode(
  private var key: Key,
  private var modifiers: Set<ShortcutModifier>,
  private var enabled: Boolean,
  private var onShortcut: () -> Unit,
) : Modifier.Node(), FocusEventModifierNode {
  private val focusManager = KeyboardFocusManager.getCurrentKeyboardFocusManager()
  private val keyEventDispatcher = KeyEventDispatcher(::dispatchKeyEvent)
  private var focused = false
  private var sawKeyDown = false
  private var handledKeyDown = false

  override fun onFocusEvent(focusState: FocusState) {
    focused = focusState.isFocused
    if (!focused) {
      sawKeyDown = false
      handledKeyDown = false
    }
  }

  override fun onAttach() {
    focusManager.addKeyEventDispatcher(keyEventDispatcher)
  }

  override fun onDetach() {
    focusManager.removeKeyEventDispatcher(keyEventDispatcher)
    super.onDetach()
  }

  fun update(key: Key, modifiers: Set<ShortcutModifier>, enabled: Boolean, onShortcut: () -> Unit) {
    this.key = key
    this.modifiers = modifiers
    this.enabled = enabled
    this.onShortcut = onShortcut
  }

  private fun dispatchKeyEvent(event: KeyEvent): Boolean {
    if (!matchesKey(event)) {
      return false
    }

    if (event.id == KeyEvent.KEY_RELEASED) {
      val shouldHandle = !sawKeyDown && focused && enabled && modifiersMatch(event)
      sawKeyDown = false
      handledKeyDown = false
      if (shouldHandle) {
        onShortcut()
      }
      return shouldHandle
    }

    if (event.id != KeyEvent.KEY_PRESSED) {
      return false
    }

    if (!sawKeyDown) {
      sawKeyDown = true
      handledKeyDown = focused && enabled && modifiersMatch(event)
      if (handledKeyDown) {
        onShortcut()
      }
    }
    return handledKeyDown
  }

  private fun matchesKey(event: KeyEvent): Boolean {
    val eventKeyLocation =
      if (event.keyLocation == KeyEvent.KEY_LOCATION_UNKNOWN) {
        KeyEvent.KEY_LOCATION_STANDARD
      } else {
        event.keyLocation
      }

    return event.keyCode == key.nativeKeyCode && eventKeyLocation == key.nativeKeyLocation
  }

  private fun modifiersMatch(event: KeyEvent): Boolean =
    matchesShortcutModifiers(
      isShiftPressed = event.isShiftDown,
      isCtrlPressed = event.isControlDown,
      isMetaPressed = event.isMetaDown,
      isAltPressed = event.isAltDown,
      modifiers = modifiers,
    )
}
