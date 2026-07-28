package co.typie.ui.utils

import androidx.compose.ui.Modifier
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.onPreviewKeyEvent

internal actual val platformModUsesMeta: Boolean = false

internal actual fun Modifier.onShortcut(
  key: Key,
  modifiers: Set<ShortcutModifier>,
  enabled: Boolean,
  onShortcut: () -> Unit,
): Modifier = onPreviewKeyEvent { event ->
  if (!enabled || !matchesShortcut(event = event, key = key, modifiers = modifiers)) {
    return@onPreviewKeyEvent false
  }

  onShortcut()
  true
}
