package co.typie.editor.viewport

import androidx.compose.ui.geometry.Size
import co.typie.editor.VerticalSpan
import co.typie.editor.ffi.ViewportAnchor
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.EditorVisibleArea
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorViewportAnchorStateTest {
  private val identity = ViewportAnchor.Node(node = "1:1", offsetX = 0f, offsetY = 0f)
  private val viewportIdentity = ViewportAnchor.Node(node = "2:1", offsetX = 0f, offsetY = 0f)

  @Test
  fun `publication keeps the anchor at the exact attached viewport point`() {
    val state = EditorViewportAnchorState()
    state.attach(identity, geometry(pointY = 200f), scrollY = 100f)

    assertEquals(
      220f,
      state.publicationScroll(
        geometry = geometry(pointY = 320f),
        currentScrollY = 100f,
        maximumScrollY = 500f,
      ),
    )
  }

  @Test
  fun `geometry change below the anchor does not move the viewport`() {
    val state = EditorViewportAnchorState()
    state.attach(identity, geometry(pointY = 200f), scrollY = 100f)

    assertEquals(
      100f,
      state.publicationScroll(
        geometry = geometry(pointY = 200f),
        currentScrollY = 100f,
        maximumScrollY = 500f,
      ),
    )
  }

  @Test
  fun `direct scroll retains identity inside cursor guard and replaces it outside`() {
    val state = EditorViewportAnchorState()
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    state.attach(identity, geometry(pointY = 200f, top = 190f, bottom = 210f), scrollY = 100f)

    assertTrue(
      state.canRetainAfterDirectScroll(
        geometry = geometry(pointY = 200f, top = 190f, bottom = 210f),
        scrollY = 80f,
        visibleArea = visibleArea,
      )
    )
    assertFalse(
      state.canRetainAfterDirectScroll(
        geometry = geometry(pointY = 200f, top = 190f, bottom = 210f),
        scrollY = 170f,
        visibleArea = visibleArea,
      )
    )
  }

  @Test
  fun `oversized rect uses its point instead of trying to fit the whole rect`() {
    val state = EditorViewportAnchorState()
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    state.attach(identity, geometry(pointY = 150f, top = 0f, bottom = 1_000f), scrollY = 0f)

    assertTrue(
      state.canRetainAfterDirectScroll(
        geometry = geometry(pointY = 150f, top = 0f, bottom = 1_000f),
        scrollY = 0f,
        visibleArea = visibleArea,
      )
    )
    assertEquals(
      0f,
      state.resizeScroll(
        geometry = geometry(pointY = 150f, top = 0f, bottom = 1_000f),
        currentScrollY = 0f,
        maximumScrollY = 700f,
        visibleArea = visibleArea,
      ),
    )
  }

  @Test
  fun `resize moves minimally only after the anchor leaves cursor guard`() {
    val state = EditorViewportAnchorState()
    state.attach(identity, geometry(pointY = 260f, top = 250f, bottom = 270f), scrollY = 100f)
    val shrunken =
      EditorVisibleArea(viewport = Size(width = 300f, height = 300f), bottomOcclusionInset = 100f)

    assertEquals(
      130f,
      state.resizeScroll(
        geometry = geometry(pointY = 260f, top = 250f, bottom = 270f),
        currentScrollY = 100f,
        maximumScrollY = 700f,
        visibleArea = shrunken,
      ),
    )
  }

  @Test
  fun `clamped publication records the achieved attachment`() {
    val state = EditorViewportAnchorState()
    state.attach(identity, geometry(pointY = 200f), scrollY = 100f)
    val candidate = geometry(pointY = 800f)

    val clamped =
      state.publicationScroll(geometry = candidate, currentScrollY = 100f, maximumScrollY = 500f)
    state.acceptGeometry(candidate, scrollY = clamped)

    assertEquals(300f, state.pointAttachmentY)
  }

  @Test
  fun `provisional selection reveal reuses the original reveal intent with measured geometry`() {
    val state = EditorViewportAnchorState()
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 400f))
    val revealOrigin =
      EditorViewportAnchorRevealOrigin(
        scrollY = 100f,
        target = EditorBringIntoViewTarget.CurrentSelectionHead,
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )
    state.attachSelection(
      identity,
      geometry(pointY = 500.5f, top = 500f, bottom = 501f),
      scrollY = 161f,
      revealOrigin = revealOrigin,
    )

    assertEquals(
      240f,
      state.publicationRevealScroll(
        geometry = geometry(pointY = 600f, top = 500f, bottom = 700f),
        currentScrollY = 161f,
        maximumScrollY = 600f,
        visibleArea = visibleArea,
        resolveReveal = { origin ->
          assertEquals(revealOrigin, origin)
          240f
        },
      ),
    )
  }

  @Test
  fun `preferred selection rect becomes active again after direct scroll returns it inside guard`() {
    val state = EditorViewportAnchorState()
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    val selectionGeometry = geometry(pointY = 200f, top = 190f, bottom = 210f)
    state.attachSelection(identity, selectionGeometry, scrollY = 100f)
    state.attachViewport(viewportIdentity, geometry(pointY = 500f), scrollY = 350f)

    assertTrue(
      state.tryReactivatePreferredSelection(
        geometry = selectionGeometry,
        scrollY = 120f,
        visibleArea = visibleArea,
      )
    )
    assertEquals(identity, state.identity)
    assertEquals(80f, state.pointAttachmentY)
  }

  @Test
  fun `point-only and oversized preferred selections do not regain priority`() {
    val state = EditorViewportAnchorState()
    val visibleArea = EditorVisibleArea(viewport = Size(width = 300f, height = 300f))
    state.attachSelection(
      identity,
      geometry(pointY = 200f, top = 0f, bottom = 1_000f),
      scrollY = 100f,
    )
    state.attachViewport(viewportIdentity, geometry(pointY = 500f), scrollY = 350f)

    assertFalse(
      state.tryReactivatePreferredSelection(
        geometry = geometry(pointY = 200f, top = 0f, bottom = 1_000f),
        scrollY = 100f,
        visibleArea = visibleArea,
      )
    )
    assertFalse(
      state.tryReactivatePreferredSelection(
        geometry = geometry(pointY = 200f),
        scrollY = 100f,
        visibleArea = visibleArea,
      )
    )
    assertEquals(viewportIdentity, state.identity)
  }

  private fun geometry(
    pointY: Float,
    top: Float? = null,
    bottom: Float? = null,
  ): EditorViewportAnchorGeometry =
    EditorViewportAnchorGeometry(
      pointY = pointY,
      rect = if (top != null && bottom != null) VerticalSpan(top, bottom) else null,
    )
}
