package co.typie.ui.utils

import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.isAltPressed
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.input.key.isMetaPressed
import androidx.compose.ui.input.key.isShiftPressed
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.type

internal enum class ShortcutModifier {
  Shift,
  Mod,
  Ctrl,
  Alt,
}

internal expect val platformModUsesMeta: Boolean

// NOTE: 플랫폼 텍스트 입력이 KeyDown을 Compose modifier보다 먼저 소비할 수 있어
// 일반 onPreviewKeyEvent만으로 처리할 수 없는 shortcut을 플랫폼별 입력 경로에서 통합
internal expect fun Modifier.onShortcut(
  key: Key,
  modifiers: Set<ShortcutModifier> = emptySet(),
  enabled: Boolean,
  onShortcut: () -> Unit,
): Modifier

internal fun matchesShortcut(
  event: KeyEvent,
  key: Key,
  modifiers: Set<ShortcutModifier> = emptySet(),
  eventType: KeyEventType = KeyEventType.KeyDown,
): Boolean {
  if (event.type != eventType || event.key != key) {
    return false
  }

  return matchesShortcutModifiers(
    isShiftPressed = event.isShiftPressed,
    isCtrlPressed = event.isCtrlPressed,
    isMetaPressed = event.isMetaPressed,
    isAltPressed = event.isAltPressed,
    modifiers = modifiers,
  )
}

internal fun matchesShortcutModifiers(
  isShiftPressed: Boolean,
  isCtrlPressed: Boolean,
  isMetaPressed: Boolean,
  isAltPressed: Boolean,
  modifiers: Set<ShortcutModifier>,
): Boolean {
  val expectsMod = ShortcutModifier.Mod in modifiers
  val expectedCtrl = ShortcutModifier.Ctrl in modifiers || (expectsMod && !platformModUsesMeta)
  val expectedMeta = expectsMod && platformModUsesMeta

  return isShiftPressed == (ShortcutModifier.Shift in modifiers) &&
    isCtrlPressed == expectedCtrl &&
    isMetaPressed == expectedMeta &&
    isAltPressed == (ShortcutModifier.Alt in modifiers)
}

/** Uses the same platform modifier policy as shortcut matching. */
internal fun shortcutLabel(key: String, vararg modifiers: ShortcutModifier): String {
  val keys = buildList {
    if (platformModUsesMeta) {
      if (ShortcutModifier.Ctrl in modifiers) add("⌃")
      if (ShortcutModifier.Alt in modifiers) add("⌥")
      if (ShortcutModifier.Shift in modifiers) add("⇧")
      if (ShortcutModifier.Mod in modifiers) add("⌘")
    } else {
      if (ShortcutModifier.Ctrl in modifiers || ShortcutModifier.Mod in modifiers) add("Ctrl")
      if (ShortcutModifier.Alt in modifiers) add("Alt")
      if (ShortcutModifier.Shift in modifiers) add("Shift")
    }
    add(if (platformModUsesMeta && key == "Enter") "↵" else key)
  }
  return keys.joinToString(if (platformModUsesMeta) "" else "+")
}
