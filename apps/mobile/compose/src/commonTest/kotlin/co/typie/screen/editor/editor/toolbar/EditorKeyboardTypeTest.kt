package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.dp
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class EditorKeyboardTypeTest {
  @Test
  fun keyboard_state_defaults_to_hidden_presentation() {
    assertEquals(
      EditorKeyboardPresentation.Hidden,
      EditorKeyboardState(EditorKeyboardType.Software).presentation,
    )
    assertNull(EditorKeyboardState(EditorKeyboardType.Software).settledImeBottom)
  }

  @Test
  fun keyboard_presentation_tracks_show_animation() {
    assertEquals(
      EditorKeyboardPresentation.Showing,
      resolveKeyboardPresentation(
        imeBottom = 120.dp,
        animationSourceBottom = 0.dp,
        animationTargetBottom = 320.dp,
      ),
    )

    assertEquals(
      EditorKeyboardPresentation.Shown(settledImeBottom = 320.dp),
      resolveKeyboardPresentation(
        imeBottom = 320.dp,
        animationSourceBottom = 0.dp,
        animationTargetBottom = 320.dp,
      ),
    )
  }

  @Test
  fun keyboard_presentation_tracks_hide_animation() {
    assertEquals(
      EditorKeyboardPresentation.Hiding,
      resolveKeyboardPresentation(
        imeBottom = 120.dp,
        animationSourceBottom = 320.dp,
        animationTargetBottom = 0.dp,
      ),
    )

    assertEquals(
      EditorKeyboardPresentation.Hidden,
      resolveKeyboardPresentation(
        imeBottom = 0.dp,
        animationSourceBottom = 0.dp,
        animationTargetBottom = 0.dp,
      ),
    )
  }

  @Test
  fun keyboard_presentation_treats_stable_visible_ime_as_shown() {
    assertEquals(
      EditorKeyboardPresentation.Shown(settledImeBottom = 320.dp),
      resolveKeyboardPresentation(
        imeBottom = 320.dp,
        animationSourceBottom = 0.dp,
        animationTargetBottom = 0.dp,
      ),
    )
  }

  @Test
  fun ime_hide_ownership_is_preserved_until_the_keyboard_is_visible_again() {
    val tracker = EditorImeHideOwnershipTracker()

    assertNull(tracker.observe(visible = true, editorInputSessionActive = false))
    assertEquals(
      EditorImeInputOwner.Other,
      tracker.observe(visible = false, editorInputSessionActive = false),
    )
    assertEquals(
      EditorImeInputOwner.Other,
      tracker.observe(visible = false, editorInputSessionActive = true),
    )
    assertNull(tracker.observe(visible = true, editorInputSessionActive = true))
    assertEquals(
      EditorImeInputOwner.Editor,
      tracker.observe(visible = false, editorInputSessionActive = true),
    )
  }

  @Test
  fun shown_keyboard_presentation_exposes_settled_ime_bottom() {
    val keyboardState =
      EditorKeyboardState(
        type = EditorKeyboardType.Software,
        presentation = EditorKeyboardPresentation.Shown(settledImeBottom = 280.dp),
      )

    assertEquals(280.dp, keyboardState.settledImeBottom)
  }

  @Test
  fun unsettled_keyboard_presentation_has_no_settled_ime_bottom() {
    assertNull(
      EditorKeyboardState(
          type = EditorKeyboardType.Software,
          presentation = EditorKeyboardPresentation.Showing,
        )
        .settledImeBottom
    )
    assertNull(
      EditorKeyboardState(
          type = EditorKeyboardType.Software,
          presentation = EditorKeyboardPresentation.Hiding,
        )
        .settledImeBottom
    )
  }

  @Test
  fun shown_keyboard_presentation_uses_animation_target_when_current_inset_overshoots() {
    assertEquals(
      EditorKeyboardPresentation.Shown(settledImeBottom = 350.dp),
      resolveKeyboardPresentation(
        imeBottom = 806.dp,
        animationSourceBottom = 0.dp,
        animationTargetBottom = 350.dp,
      ),
    )
  }

  @Test
  fun trusted_ime_bottom_ignores_raw_inset_when_keyboard_state_does_not_use_ime() {
    assertEquals(
      0.dp,
      trustedImeBottomInset(
        rawImeBottom = 320.dp,
        keyboardState = EditorKeyboardState(type = EditorKeyboardType.Hardware),
      ),
    )
  }

  @Test
  fun trusted_ime_bottom_bounds_refocus_overshoot_to_settled_inset() {
    assertEquals(
      350.dp,
      trustedImeBottomInset(
        rawImeBottom = 806.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Software,
            presentation = EditorKeyboardPresentation.Shown(settledImeBottom = 350.dp),
          ),
      ),
    )
  }

  @Test
  fun trusted_ime_bottom_preserves_unsettled_live_inset() {
    assertEquals(
      120.dp,
      trustedImeBottomInset(
        rawImeBottom = 120.dp,
        keyboardState =
          EditorKeyboardState(
            type = EditorKeyboardType.Software,
            presentation = EditorKeyboardPresentation.Showing,
          ),
      ),
    )
  }
}
