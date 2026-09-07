@file:OptIn(androidx.compose.ui.InternalComposeUiApi::class)

package co.typie.editor

import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import co.typie.platform.Platform
import kotlin.test.Test
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class KeyboardBindingTest {
  @Test
  fun iOSLeavesHorizontalMovementAndSelectionToNativeInput() {
    val bindings = createBindings(Platform.iOS)

    for (key in listOf(Key.DirectionLeft, Key.DirectionRight)) {
      for (shift in listOf(true, false)) {
        val event = KeyEvent(key, KeyEventType.KeyDown, isShiftPressed = shift)
        assertNull(
          bindings.find { matchesKeyBinding(it, Platform.iOS, event) },
          "$key shift=$shift must reach native input for repeat and selection collapse",
        )
      }
    }
  }

  @Test
  fun iOSKeepsVisualNavigationForwardDeleteAndModifiedHorizontalCommands() {
    val bindings = createBindings(Platform.iOS)
    val events =
      listOf(
        KeyEvent(Key.DirectionUp, KeyEventType.KeyDown),
        KeyEvent(Key.DirectionDown, KeyEventType.KeyDown, isShiftPressed = true),
        KeyEvent(Key.Delete, KeyEventType.KeyDown),
        KeyEvent(Key.DirectionLeft, KeyEventType.KeyDown, isAltPressed = true),
        KeyEvent(Key.DirectionRight, KeyEventType.KeyDown, isMetaPressed = true),
      )

    for (event in events) {
      assertNotNull(bindings.find { matchesKeyBinding(it, Platform.iOS, event) })
    }
  }

  @Test
  fun otherPlatformsKeepHorizontalMovementAndSelectionBindings() {
    for (platform in listOf(Platform.Android, Platform.Desktop)) {
      val bindings = createBindings(platform)
      for (key in listOf(Key.DirectionLeft, Key.DirectionRight)) {
        for (shift in listOf(false, true)) {
          val event = KeyEvent(key, KeyEventType.KeyDown, isShiftPressed = shift)
          assertNotNull(bindings.find { matchesKeyBinding(it, platform, event) })
        }
      }
    }
  }
}
