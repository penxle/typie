package co.typie.screen.editor.editor.overlay

import co.typie.editor.EditorZoomLandmark
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorZoomIndicatorStateTest {
  @Test
  fun `initial unit zoom stays hidden in touch only state`() {
    val state = EditorZoomIndicatorState()

    state.updateZoom(enabled = true, displayZoom = 1f, landmark = EditorZoomLandmark.Unit)

    assertFalse(state.visible)
  }

  @Test
  fun `unit zoom stays hidden when initial layout enables the indicator`() {
    val state = EditorZoomIndicatorState()

    state.updateZoom(enabled = false, displayZoom = 1f, landmark = null)
    state.onPanePointerEnter(landmark = null)
    state.updateZoom(enabled = true, displayZoom = 1f, landmark = EditorZoomLandmark.Unit)

    assertFalse(state.visible)
    assertEquals(null, state.announcedLandmark)
  }

  @Test
  fun `initial non unit zoom remains transiently visible`() {
    val state = EditorZoomIndicatorState()

    state.updateZoom(enabled = true, displayZoom = 1.25f, landmark = null)

    assertTrue(state.visible)
    state.expireVisibility(state.visibilityRequest)
    assertFalse(state.visible)
  }

  @Test
  fun `first non touch pointer entry enables controls and reveals non unit zoom`() {
    val state = settledState(landmark = null, displayZoom = 1.25f)

    state.onPanePointerEnter(landmark = null)

    assertTrue(state.visible)
  }

  @Test
  fun `unit pointer entry enables controls without revealing the indicator`() {
    val state = settledState(landmark = EditorZoomLandmark.Unit, displayZoom = 1f)

    state.onPanePointerEnter(landmark = EditorZoomLandmark.Unit)

    assertFalse(state.visible)
  }

  @Test
  fun `indicator hover holds visibility after the transient request expires`() {
    val state = settledState(landmark = null, displayZoom = 1.25f)
    state.onPanePointerEnter(landmark = null)
    val request = state.visibilityRequest

    state.onIndicatorPointerEnter()
    state.expireVisibility(request)

    assertTrue(state.visible)
    state.onIndicatorPointerExit()
    assertTrue(state.visible)
    state.expireVisibility(state.visibilityRequest)
    assertFalse(state.visible)
  }

  @Test
  fun `focus exit keeps the indicator visible for one more transient request`() {
    val state = settledState(landmark = null, displayZoom = 1.25f)

    state.onFocusChanged(true)
    assertTrue(state.visible)

    state.onFocusChanged(false)
    assertTrue(state.visible)

    state.expireVisibility(state.visibilityRequest)
    assertFalse(state.visible)
  }

  @Test
  fun `landmark labels are temporary and value hover always describes current state`() {
    val state = settledState(landmark = null, displayZoom = 1.25f)

    state.updateZoom(enabled = true, displayZoom = 1f, landmark = EditorZoomLandmark.Unit)

    assertEquals("원본", state.displayText(EditorZoomLandmark.Unit, 100))
    state.expireLandmark(state.landmarkRequest)
    assertEquals("100%", state.displayText(EditorZoomLandmark.Unit, 100))

    state.onValuePointerEnter()
    assertEquals("원본", state.displayText(EditorZoomLandmark.Unit, 100))
    state.onValuePointerExit()
    assertEquals("100%", state.displayText(EditorZoomLandmark.Unit, 100))
  }

  @Test
  fun `snap feedback starts only when applied state enters a named landmark`() {
    val state = settledState(landmark = null, displayZoom = 1.25f)

    assertEquals(0, state.snapFeedbackRequest)

    state.updateZoom(enabled = true, displayZoom = 1f, landmark = EditorZoomLandmark.Unit)
    assertEquals(1, state.snapFeedbackRequest)

    state.updateZoom(enabled = true, displayZoom = 1f, landmark = EditorZoomLandmark.Unit)
    assertEquals(1, state.snapFeedbackRequest)

    state.updateZoom(enabled = true, displayZoom = 0.95f, landmark = null)
    assertEquals(1, state.snapFeedbackRequest)
    assertEquals(EditorZoomLandmark.Unit, state.snapFeedbackLandmark)

    state.updateZoom(enabled = true, displayZoom = 1f, landmark = EditorZoomLandmark.Unit)
    assertEquals(2, state.snapFeedbackRequest)
  }

  @Test
  fun `snap feedback identifies the actual applied bound landmark`() {
    val state = settledState(landmark = EditorZoomLandmark.Unit, displayZoom = 1f)

    state.updateZoom(enabled = true, displayZoom = 2f, landmark = EditorZoomLandmark.Maximum)

    assertEquals(1, state.snapFeedbackRequest)
    assertEquals(EditorZoomLandmark.Maximum, state.snapFeedbackLandmark)
  }

  @Test
  fun `unchanged boundary attempts restart only the matching label and visibility`() {
    val minimum = settledState(landmark = EditorZoomLandmark.Minimum, displayZoom = 0.2f)
    val minimumSnapRequest = minimum.snapFeedbackRequest

    minimum.onBoundaryAttempt(EditorZoomLandmark.Minimum)

    assertTrue(minimum.visible)
    assertEquals(EditorZoomLandmark.Minimum, minimum.announcedLandmark)
    assertEquals(minimumSnapRequest, minimum.snapFeedbackRequest)

    val maximum = settledState(landmark = EditorZoomLandmark.Maximum, displayZoom = 2f)
    val maximumSnapRequest = maximum.snapFeedbackRequest

    maximum.onBoundaryAttempt(EditorZoomLandmark.Maximum)

    assertEquals(EditorZoomLandmark.Maximum, maximum.announcedLandmark)
    assertEquals(maximumSnapRequest, maximum.snapFeedbackRequest)
  }

  @Test
  fun `overshoot announces once and recovery restarts the boundary label with its flash`() {
    val state = settledState(landmark = null, displayZoom = 1.25f)
    val initialLandmarkRequest = state.landmarkRequest
    val initialSnapRequest = state.snapFeedbackRequest

    state.updateZoom(
      enabled = true,
      displayZoom = 0.15f,
      indicatorZoom = 0.2f,
      landmark = EditorZoomLandmark.Minimum,
    )
    assertEquals(initialLandmarkRequest + 1, state.landmarkRequest)
    assertEquals(EditorZoomLandmark.Minimum, state.announcedLandmark)
    assertEquals(initialSnapRequest, state.snapFeedbackRequest)

    state.updateZoom(
      enabled = true,
      displayZoom = 0.14f,
      indicatorZoom = 0.2f,
      landmark = EditorZoomLandmark.Minimum,
    )
    assertEquals(initialLandmarkRequest + 1, state.landmarkRequest)

    state.expireLandmark(state.landmarkRequest)
    assertEquals(EditorZoomLandmark.Minimum, state.announcedLandmark)
    state.expireVisibility(state.visibilityRequest)
    assertTrue(state.visible)
    state.updateZoom(
      enabled = true,
      displayZoom = 0.2f,
      indicatorZoom = 0.2f,
      landmark = EditorZoomLandmark.Minimum,
    )
    assertEquals(initialLandmarkRequest + 2, state.landmarkRequest)
    state.expireLandmark(state.landmarkRequest)
    assertEquals(null, state.announcedLandmark)
    assertEquals(initialSnapRequest + 1, state.snapFeedbackRequest)
    assertEquals(EditorZoomLandmark.Minimum, state.snapFeedbackLandmark)

    state.updateZoom(
      enabled = true,
      displayZoom = 0.15f,
      indicatorZoom = 0.2f,
      landmark = EditorZoomLandmark.Minimum,
    )
    assertEquals(initialLandmarkRequest + 3, state.landmarkRequest)
    assertEquals(initialSnapRequest + 1, state.snapFeedbackRequest)
  }

  @Test
  fun `overshoot precedence handles side changes and recovery into another landmark`() {
    val state = settledState(landmark = null, displayZoom = 1.25f)

    state.updateZoom(
      enabled = true,
      displayZoom = 0.15f,
      indicatorZoom = 0.2f,
      landmark = EditorZoomLandmark.Minimum,
    )
    state.updateZoom(
      enabled = true,
      displayZoom = 2.1f,
      indicatorZoom = 2f,
      landmark = EditorZoomLandmark.Maximum,
    )
    assertEquals(EditorZoomLandmark.Maximum, state.announcedLandmark)
    assertEquals(0, state.snapFeedbackRequest)

    state.expireLandmark(state.landmarkRequest)
    state.updateZoom(
      enabled = true,
      displayZoom = 1f,
      indicatorZoom = 1f,
      landmark = EditorZoomLandmark.Unit,
    )
    assertEquals(EditorZoomLandmark.Maximum, state.announcedLandmark)
    assertEquals(1, state.snapFeedbackRequest)
    assertEquals(EditorZoomLandmark.Maximum, state.snapFeedbackLandmark)
  }

  @Test
  fun `reset clears late visibility requests`() {
    val state = settledState(landmark = null, displayZoom = 1.25f)
    state.onPanePointerEnter(landmark = null)
    state.onIndicatorPointerEnter()

    state.reset()

    assertFalse(state.visible)
    assertEquals(null, state.announcedLandmark)
  }

  private fun settledState(
    landmark: EditorZoomLandmark?,
    displayZoom: Float,
  ): EditorZoomIndicatorState =
    EditorZoomIndicatorState().also { state ->
      state.updateZoom(enabled = true, displayZoom = displayZoom, landmark = landmark)
      state.expireVisibility(state.visibilityRequest)
      state.expireLandmark(state.landmarkRequest)
    }
}
