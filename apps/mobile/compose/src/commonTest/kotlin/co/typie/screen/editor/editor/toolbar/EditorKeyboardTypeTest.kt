package co.typie.screen.editor.editor.toolbar

import androidx.compose.ui.unit.dp
import co.typie.platform.SoftwareKeyboardInteractionResolution
import co.typie.platform.SoftwareKeyboardInteractionState
import co.typie.platform.SoftwareKeyboardPresentationController
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
  fun auxiliary_visible_ime_keeps_other_ownership_when_editor_refocuses_at_hide() {
    val tracker = EditorImeHideOwnershipTracker()

    tracker.observeVisibleOwner(editorInputSessionActive = false)

    assertEquals(EditorImeInputOwner.Other, tracker.beginHide())
  }

  @Test
  fun visible_ime_transfers_to_editor_before_a_later_hide() {
    val tracker = EditorImeHideOwnershipTracker()

    tracker.observeVisibleOwner(editorInputSessionActive = false)
    tracker.observeVisibleOwner(editorInputSessionActive = true)

    assertEquals(EditorImeInputOwner.Editor, tracker.beginHide())
  }

  @Test
  fun ime_hide_ownership_is_preserved_until_the_keyboard_is_visible_again() {
    val tracker = EditorImeHideOwnershipTracker()

    tracker.observeVisibleOwner(editorInputSessionActive = false)
    assertEquals(EditorImeInputOwner.Other, tracker.beginHide())
    assertEquals(EditorImeInputOwner.Other, tracker.beginHide())

    tracker.observeVisibleOwner(editorInputSessionActive = true)
    assertEquals(EditorImeInputOwner.Editor, tracker.beginHide())
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

  @Test
  fun active_keyboard_interaction_retains_the_last_shown_semantics() {
    val resolver = EditorKeyboardInteractionResolver()
    val shown = shownKeyboardState(version = 3, owner = EditorImeInputOwner.Editor)
    resolver.resolve(shown, SoftwareKeyboardInteractionState())

    val resolved =
      resolver.resolve(
        nativeState =
          hiddenKeyboardState(
            presentation = EditorKeyboardPresentation.Hiding,
            version = 4,
            owner = EditorImeInputOwner.Other,
          ),
        interactionState =
          SoftwareKeyboardInteractionState(activeInteractionId = 1L, hiddenProgress = 1f),
      )

    assertEquals(EditorKeyboardPresentation.Shown(320.dp), resolved.presentation)
    assertEquals(true, resolved.imeFrameVisible)
    assertEquals(3, resolved.imeHideEventVersion)
    assertEquals(EditorImeInputOwner.Editor, resolved.imeHideEventOwner)
  }

  @Test
  fun acquiring_interaction_retains_shown_semantics_before_the_factory_runs() {
    val resolver = EditorKeyboardInteractionResolver()
    val shown = shownKeyboardState(version = 3, owner = EditorImeInputOwner.Editor)
    resolver.resolve(shown, SoftwareKeyboardInteractionState())
    lateinit var controller: SoftwareKeyboardPresentationController
    var resolvedDuringAcquisition: EditorKeyboardState? = null
    controller = SoftwareKeyboardPresentationController {
      resolvedDuringAcquisition =
        resolver.resolve(
          nativeState =
            hiddenKeyboardState(
              presentation = EditorKeyboardPresentation.Hiding,
              version = 4,
              owner = EditorImeInputOwner.Other,
            ),
          interactionState = controller.interactionState,
        )
      null
    }

    assertNull(controller.acquire())

    val resolved = requireNotNull(resolvedDuringAcquisition)
    assertEquals(EditorKeyboardPresentation.Shown(320.dp), resolved.presentation)
    assertEquals(3, resolved.imeHideEventVersion)
    assertEquals(EditorImeInputOwner.Editor, resolved.imeHideEventOwner)
  }

  @Test
  fun shown_resolution_discards_transient_native_hide_version() {
    val resolver = EditorKeyboardInteractionResolver()
    val shown = shownKeyboardState(version = 3)
    resolver.resolve(shown, SoftwareKeyboardInteractionState())
    resolver.resolve(
      hiddenKeyboardState(presentation = EditorKeyboardPresentation.Hiding, version = 4),
      SoftwareKeyboardInteractionState(activeInteractionId = 1L, hiddenProgress = 0.7f),
    )

    val stillRetained =
      resolver.resolve(
        hiddenKeyboardState(presentation = EditorKeyboardPresentation.Hidden, version = 4),
        resolvedInteraction(SoftwareKeyboardInteractionResolution.Shown),
      )
    val settledShown =
      resolver.resolve(
        shownKeyboardState(version = 4),
        resolvedInteraction(SoftwareKeyboardInteractionResolution.Shown),
      )
    val laterRealHide =
      resolver.resolve(
        hiddenKeyboardState(
          presentation = EditorKeyboardPresentation.Hiding,
          version = 5,
          owner = EditorImeInputOwner.Editor,
        ),
        resolvedInteraction(SoftwareKeyboardInteractionResolution.Shown),
      )

    assertEquals(EditorKeyboardPresentation.Shown(320.dp), stillRetained.presentation)
    assertEquals(3, stillRetained.imeHideEventVersion)
    assertEquals(EditorKeyboardPresentation.Shown(320.dp), settledShown.presentation)
    assertEquals(3, settledShown.imeHideEventVersion)
    assertEquals(EditorKeyboardPresentation.Hiding, laterRealHide.presentation)
    assertEquals(4, laterRealHide.imeHideEventVersion)
  }

  @Test
  fun hidden_resolution_publishes_the_native_hide_once_it_reaches_the_endpoint() {
    val resolver = EditorKeyboardInteractionResolver()
    resolver.resolve(shownKeyboardState(version = 2), SoftwareKeyboardInteractionState())
    resolver.resolve(
      hiddenKeyboardState(presentation = EditorKeyboardPresentation.Hiding, version = 3),
      SoftwareKeyboardInteractionState(activeInteractionId = 1L, hiddenProgress = 1f),
    )

    val resolved =
      resolver.resolve(
        hiddenKeyboardState(
          presentation = EditorKeyboardPresentation.Hiding,
          version = 3,
          owner = EditorImeInputOwner.Editor,
        ),
        resolvedInteraction(SoftwareKeyboardInteractionResolution.Hidden),
      )
    val settled =
      resolver.resolve(
        hiddenKeyboardState(
          presentation = EditorKeyboardPresentation.Hidden,
          version = 3,
          owner = EditorImeInputOwner.Editor,
        ),
        resolvedInteraction(SoftwareKeyboardInteractionResolution.Hidden),
      )

    assertEquals(EditorKeyboardPresentation.Hiding, resolved.presentation)
    assertEquals(3, resolved.imeHideEventVersion)
    assertEquals(EditorImeInputOwner.Editor, resolved.imeHideEventOwner)
    assertEquals(EditorKeyboardPresentation.Hidden, settled.presentation)
    assertEquals(3, settled.imeHideEventVersion)
  }

  @Test
  fun aborted_interaction_releases_to_the_current_native_state() {
    val resolver = EditorKeyboardInteractionResolver()
    resolver.resolve(shownKeyboardState(version = 1), SoftwareKeyboardInteractionState())
    resolver.resolve(
      hiddenKeyboardState(presentation = EditorKeyboardPresentation.Hiding, version = 2),
      SoftwareKeyboardInteractionState(activeInteractionId = 1L, hiddenProgress = 0.5f),
    )
    val nativeHidden =
      hiddenKeyboardState(
        presentation = EditorKeyboardPresentation.Hidden,
        version = 2,
        owner = EditorImeInputOwner.Other,
      )

    assertEquals(
      nativeHidden,
      resolver.resolve(
        nativeState = nativeHidden,
        interactionState = resolvedInteraction(SoftwareKeyboardInteractionResolution.Aborted),
      ),
    )
  }

  @Test
  fun later_interaction_captures_a_fresh_shown_state() {
    val resolver = EditorKeyboardInteractionResolver()
    resolver.resolve(shownKeyboardState(version = 1), SoftwareKeyboardInteractionState())
    resolver.resolve(
      shownKeyboardState(version = 1),
      SoftwareKeyboardInteractionState(activeInteractionId = 1L),
    )
    resolver.resolve(
      shownKeyboardState(version = 1, settledImeBottom = 320.dp),
      resolvedInteraction(SoftwareKeyboardInteractionResolution.Shown),
    )

    val freshShown = shownKeyboardState(version = 1, settledImeBottom = 360.dp)
    resolver.resolve(freshShown, resolvedInteraction(SoftwareKeyboardInteractionResolution.Shown))
    val retained =
      resolver.resolve(
        hiddenKeyboardState(presentation = EditorKeyboardPresentation.Hidden, version = 2),
        SoftwareKeyboardInteractionState(
          activeInteractionId = 2L,
          hiddenProgress = 1f,
          resolutionVersion = 1L,
          lastResolution = SoftwareKeyboardInteractionResolution.Shown,
        ),
      )

    assertEquals(EditorKeyboardPresentation.Shown(360.dp), retained.presentation)
    assertEquals(1, retained.imeHideEventVersion)
  }
}

private fun shownKeyboardState(
  version: Int,
  owner: EditorImeInputOwner? = null,
  settledImeBottom: androidx.compose.ui.unit.Dp = 320.dp,
) =
  EditorKeyboardState(
    type = EditorKeyboardType.Software,
    imeFrameVisible = true,
    imeHideEventVersion = version,
    imeHideEventOwner = owner,
    presentation = EditorKeyboardPresentation.Shown(settledImeBottom),
  )

private fun hiddenKeyboardState(
  presentation: EditorKeyboardPresentation,
  version: Int,
  owner: EditorImeInputOwner? = null,
) =
  EditorKeyboardState(
    type = EditorKeyboardType.Software,
    imeFrameVisible = false,
    imeHideEventVersion = version,
    imeHideEventOwner = owner,
    presentation = presentation,
  )

private fun resolvedInteraction(resolution: SoftwareKeyboardInteractionResolution) =
  SoftwareKeyboardInteractionState(resolutionVersion = 1L, lastResolution = resolution)
