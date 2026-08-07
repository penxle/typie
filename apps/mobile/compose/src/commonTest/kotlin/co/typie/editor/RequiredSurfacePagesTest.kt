package co.typie.editor

import androidx.compose.ui.geometry.Size
import co.typie.editor.scroll.EditorBringIntoViewPolicy
import co.typie.editor.scroll.EditorVisibleArea
import co.typie.editor.scroll.resolveEditorAutoScrollPolicy
import co.typie.editor.scroll.resolveInstantRevealPreparationViewports
import co.typie.editor.scroll.resolveKeepVisibleScrollOffset
import co.typie.editor.scroll.resolveTypewriterScrollOffset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class RequiredSurfacePagesTest {
  @Test
  fun `empty-document`() {
    assertEquals(
      emptySet(),
      requiredSurfacePages(
        pages = emptyList(),
        currentViewport = VerticalSpan(0f, 100f),
        activePages = setOf(0),
        preparationViewports = listOf(VerticalSpan(0f, 100f)),
      ),
    )
  }

  @Test
  fun `ignores invalid geometry`() {
    val pages = pages(0f, 100f, 200f)

    assertEquals(
      emptySet(),
      requiredSurfacePages(
        pages = pages,
        currentViewport = VerticalSpan(Float.NaN, 100f),
        activePages = setOf(1),
        preparationViewports = listOf(VerticalSpan(300f, 200f)),
      ),
    )
  }

  @Test
  fun `current-viewport-acquire-one-height`() {
    assertEquals(
      setOf(1, 2, 3),
      requiredSurfacePages(
        pages = pages(0f, 100f, 200f, 300f, 400f, 500f),
        currentViewport = VerticalSpan(200f, 300f),
      ),
    )
  }

  @Test
  fun `active-target-release-one-and-a-half-heights`() {
    val pages =
      listOf(
        SurfacePageSpan(page = 0, top = 0f, bottom = 50f),
        SurfacePageSpan(page = 1, top = 50f, bottom = 100f),
        SurfacePageSpan(page = 2, top = 100f, bottom = 200f),
        SurfacePageSpan(page = 3, top = 200f, bottom = 300f),
        SurfacePageSpan(page = 4, top = 300f, bottom = 400f),
        SurfacePageSpan(page = 5, top = 400f, bottom = 450f),
        SurfacePageSpan(page = 6, top = 450f, bottom = 500f),
      )

    assertEquals(
      setOf(1, 2, 3, 4, 5),
      requiredSurfacePages(
        pages = pages,
        currentViewport = VerticalSpan(200f, 300f),
        activePages = setOf(0, 1, 5, 6),
      ),
    )
  }

  @Test
  fun `preparation viewport uses the same acquire range as the destination viewport`() {
    val pages =
      listOf(
        SurfacePageSpan(page = 0, top = 0f, bottom = 100f),
        SurfacePageSpan(page = 1, top = 120f, bottom = 220f),
        SurfacePageSpan(page = 2, top = 240f, bottom = 340f),
      )

    assertEquals(
      setOf(0, 1),
      requiredSurfacePages(pages = pages, preparationViewports = listOf(VerticalSpan(100f, 120f))),
    )
    assertEquals(
      setOf(0, 1),
      requiredSurfacePages(pages = pages, preparationViewports = listOf(VerticalSpan(99f, 120f))),
    )
    assertEquals(
      setOf(0, 1),
      requiredSurfacePages(pages = pages, preparationViewports = listOf(VerticalSpan(100f, 121f))),
    )
  }

  @Test
  fun `zoomed-origin-and-gaps`() {
    val alreadyDerivedPageSpans =
      listOf(
        SurfacePageSpan(page = 0, top = 50f, bottom = 250f),
        SurfacePageSpan(page = 1, top = 290f, bottom = 490f),
      )

    assertEquals(
      setOf(0, 1),
      requiredSurfacePages(
        pages = alreadyDerivedPageSpans,
        preparationViewports = listOf(VerticalSpan(245f, 295f)),
      ),
    )
  }

  @Test
  fun `out-of-range-active-pages`() {
    assertEquals(
      setOf(0, 1, 2),
      requiredSurfacePages(
        pages = pages(0f, 100f, 200f, 300f),
        currentViewport = VerticalSpan(0f, 100f),
        activePages = setOf(-1, 2, 3, 100),
      ),
    )
  }

  @Test
  fun `one-to-three-append`() {
    val viewport = VerticalSpan(100f, 200f)

    assertEquals(
      setOf(0),
      requiredSurfacePages(
        pages = pages(0f, 100f),
        currentViewport = viewport,
        activePages = setOf(0),
      ),
    )
    assertEquals(
      setOf(0, 1, 2),
      requiredSurfacePages(
        pages = pages(0f, 100f, 200f, 300f),
        currentViewport = viewport,
        activePages = setOf(0),
      ),
    )
  }

  @Test
  fun `four-to-one-shrink`() {
    assertEquals(
      setOf(0),
      requiredSurfacePages(
        pages = pages(0f, 100f),
        currentViewport = VerticalSpan(100f, 200f),
        activePages = setOf(0, 1, 2, 3),
      ),
    )
  }

  @Test
  fun `long-document-bounded-count`() {
    val pages = (0 until 100).map { page -> SurfacePageSpan(page, page * 100f, (page + 1) * 100f) }

    assertEquals(
      setOf(49, 50, 51),
      requiredSurfacePages(pages = pages, currentViewport = VerticalSpan(5_000f, 5_100f)),
    )
  }

  @Test
  fun `disjoint-preparation-viewports-do-not-fill-gaps`() {
    val pages = (0 until 10).map { page -> SurfacePageSpan(page, page * 100f, (page + 1) * 100f) }

    assertEquals(
      setOf(2, 8),
      requiredSurfacePages(
        pages = pages,
        preparationViewports = listOf(VerticalSpan(210f, 220f), VerticalSpan(810f, 820f)),
      ),
    )
  }

  @Test
  fun `instant preparation covers the first destination viewport requirements`() {
    val pageSpans = pages(0f, 100f, 200f, 300f, 400f, 500f, 600f)
    val destinationViewport = VerticalSpan(300f, 400f)
    val prepared =
      requiredSurfacePages(
        pages = pageSpans,
        currentViewport = VerticalSpan(0f, 100f),
        activePages = setOf(0, 1),
        preparationViewports = listOf(destinationViewport),
      )
    val firstDestinationRequirements =
      requiredSurfacePages(
        pages = pageSpans,
        currentViewport = destinationViewport,
        activePages = prepared,
      )

    assertTrue(
      prepared.containsAll(firstDestinationRequirements),
      "preparation=$prepared destination=$firstDestinationRequirements",
    )
  }

  @Test
  fun `instant preparation contains every destination supported by the production scroll resolvers`() {
    val visibleArea =
      EditorVisibleArea(
        viewport = Size(width = 800f, height = 400f),
        topInset = 10f,
        bottomOcclusionInset = 20f,
      )
    val target = VerticalSpan(top = 500f, bottom = 650f)
    val preparation =
      resolveInstantRevealPreparationViewports(
        currentScroll = 350f,
        viewportHeight = 400f,
        maximumScrollY = 1_000f,
        target = target,
        visibleArea = visibleArea,
        autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea),
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )

    val exactDestinations =
      listOf(0f, 350f, 800f).map { currentScroll ->
        resolveKeepVisibleScrollOffset(
          currentScroll = currentScroll,
          targetTopInContent = target.top,
          targetBottomInContent = target.bottom,
          visibleArea = visibleArea,
        ) ?: currentScroll
      }

    exactDestinations.forEach { scrollY ->
      assertTrue(
        VerticalSpan(scrollY, scrollY + visibleArea.viewport.height) in preparation,
        "destination=$scrollY preparation=$preparation",
      )
    }
  }

  @Test
  fun `instant preparation covers a target taller than the visible range`() {
    val visibleArea =
      EditorVisibleArea(
        viewport = Size(width = 800f, height = 400f),
        topInset = 20f,
        bottomOcclusionInset = 30f,
      )
    val target = VerticalSpan(top = 600f, bottom = 1_100f)
    for (currentScroll in listOf(0f, 600f, 1_500f)) {
      val preparation =
        resolveInstantRevealPreparationViewports(
          currentScroll = currentScroll,
          viewportHeight = visibleArea.viewport.height,
          maximumScrollY = 1_600f,
          target = target,
          visibleArea = visibleArea,
          autoScrollPolicy = resolveEditorAutoScrollPolicy(visibleArea),
          policy = EditorBringIntoViewPolicy.CursorGuard,
        )
      val destination =
        resolveKeepVisibleScrollOffset(
          currentScroll = currentScroll,
          targetTopInContent = target.top,
          targetBottomInContent = target.bottom,
          visibleArea = visibleArea,
        ) ?: currentScroll
      assertTrue(
        VerticalSpan(destination, destination + visibleArea.viewport.height) in preparation,
        "destination=$destination preparation=$preparation",
      )
    }
  }

  @Test
  fun `instant preparation uses production clamp centered fallback and typewriter alignment`() {
    val clamped =
      resolveInstantRevealPreparationViewports(
        currentScroll = 200f,
        viewportHeight = 400f,
        maximumScrollY = 250f,
        target = VerticalSpan(top = 500f, bottom = 650f),
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 800f, height = 400f),
            topInset = 10f,
            bottomOcclusionInset = 20f,
          ),
        autoScrollPolicy =
          resolveEditorAutoScrollPolicy(
            EditorVisibleArea(
              viewport = Size(width = 800f, height = 400f),
              topInset = 10f,
              bottomOcclusionInset = 20f,
            )
          ),
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )
    assertEquals(setOf(VerticalSpan(250f, 650f)), clamped.toSet())

    val narrowVisibleArea =
      EditorVisibleArea(
        viewport = Size(width = 800f, height = 180f),
        topInset = 70f,
        bottomOcclusionInset = 70f,
      )
    val centeredTarget = VerticalSpan(top = 500f, bottom = 520f)
    val centered =
      resolveInstantRevealPreparationViewports(
        currentScroll = 0f,
        viewportHeight = 180f,
        maximumScrollY = 1_000f,
        target = centeredTarget,
        visibleArea = narrowVisibleArea,
        autoScrollPolicy = resolveEditorAutoScrollPolicy(narrowVisibleArea),
        policy = EditorBringIntoViewPolicy.CursorGuard,
      )
    val centeredDestination =
      requireNotNull(
        resolveKeepVisibleScrollOffset(
          currentScroll = 0f,
          targetTopInContent = centeredTarget.top,
          targetBottomInContent = centeredTarget.bottom,
          visibleArea = narrowVisibleArea,
        )
      )
    assertEquals(listOf(VerticalSpan(centeredDestination, centeredDestination + 180f)), centered)

    val typewriterVisibleArea = EditorVisibleArea(viewport = Size(width = 800f, height = 400f))
    val typewriterTarget = VerticalSpan(top = 800f, bottom = 820f)
    val typewriter =
      resolveInstantRevealPreparationViewports(
        currentScroll = 0f,
        viewportHeight = 400f,
        maximumScrollY = 1_500f,
        target = typewriterTarget,
        visibleArea = typewriterVisibleArea,
        autoScrollPolicy =
          resolveEditorAutoScrollPolicy(
            visibleArea = typewriterVisibleArea,
            typewriterEnabled = true,
            typewriterPosition = 0.5f,
          ),
        policy = EditorBringIntoViewPolicy.Typewriter,
      )
    val typewriterDestination =
      requireNotNull(
        resolveTypewriterScrollOffset(
          currentScroll = 0f,
          targetTopInContent = typewriterTarget.top,
          targetBottomInContent = typewriterTarget.bottom,
          visibleArea = typewriterVisibleArea,
          position = 0.5f,
        )
      )
    assertEquals(
      listOf(VerticalSpan(typewriterDestination, typewriterDestination + 400f)),
      typewriter,
    )

    val oversizedTypewriterTarget = VerticalSpan(top = 600f, bottom = 1_100f)
    val oversizedTypewriter =
      resolveInstantRevealPreparationViewports(
        currentScroll = 0f,
        viewportHeight = 400f,
        maximumScrollY = 1_600f,
        target = oversizedTypewriterTarget,
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 800f, height = 400f),
            topInset = 20f,
            bottomOcclusionInset = 30f,
          ),
        autoScrollPolicy =
          resolveEditorAutoScrollPolicy(
            visibleArea =
              EditorVisibleArea(
                viewport = Size(width = 800f, height = 400f),
                topInset = 20f,
                bottomOcclusionInset = 30f,
              ),
            typewriterEnabled = true,
            typewriterPosition = 0.5f,
          ),
        policy = EditorBringIntoViewPolicy.Typewriter,
      )
    assertEquals(
      listOf(VerticalSpan(0f, 400f), VerticalSpan(520f, 920f), VerticalSpan(790f, 1_190f)),
      oversizedTypewriter,
    )

    val documentEdgeTarget = VerticalSpan(top = 10f, bottom = 500f)
    val documentEdge =
      resolveInstantRevealPreparationViewports(
        currentScroll = 800f,
        viewportHeight = 400f,
        maximumScrollY = 1_600f,
        target = documentEdgeTarget,
        visibleArea =
          EditorVisibleArea(
            viewport = Size(width = 800f, height = 400f),
            topInset = 20f,
            bottomOcclusionInset = 30f,
          ),
        autoScrollPolicy =
          resolveEditorAutoScrollPolicy(
            visibleArea =
              EditorVisibleArea(
                viewport = Size(width = 800f, height = 400f),
                topInset = 20f,
                bottomOcclusionInset = 30f,
              ),
            typewriterEnabled = true,
            typewriterPosition = 0.5f,
          ),
        policy = EditorBringIntoViewPolicy.Typewriter,
      )
    assertEquals(
      listOf(VerticalSpan(800f, 1_200f), VerticalSpan(0f, 400f), VerticalSpan(190f, 590f)),
      documentEdge,
    )
  }

  @Test
  fun `bounded instant preparation covers each destination cohort without filling disjoint gaps`() {
    val pageSpans =
      (0 until 30).map { page -> SurfacePageSpan(page, page * 100f, (page + 1) * 100f) }
    val visibleArea = EditorVisibleArea(viewport = Size(width = 800f, height = 100f))
    val preparationViewports =
      listOf(VerticalSpan(300f, 320f), VerticalSpan(2_300f, 2_320f)).flatMap { target ->
        resolveInstantRevealPreparationViewports(
          currentScroll = 0f,
          viewportHeight = 100f,
          maximumScrollY = 2_900f,
          target = target,
          visibleArea = visibleArea,
          autoScrollPolicy =
            resolveEditorAutoScrollPolicy(
              visibleArea = visibleArea,
              typewriterEnabled = true,
              typewriterPosition = 0.5f,
            ),
          policy = EditorBringIntoViewPolicy.Typewriter,
        )
      }
    val prepared =
      requiredSurfacePages(
        pages = pageSpans,
        currentViewport = VerticalSpan(0f, 100f),
        preparationViewports = preparationViewports,
      )

    preparationViewports.forEach { destination ->
      val destinationRequired =
        requiredSurfacePages(pages = pageSpans, currentViewport = destination)
      assertTrue(
        prepared.containsAll(destinationRequired),
        "destination=$destination prepared=$prepared required=$destinationRequired",
      )
    }
    assertFalse(
      12 in prepared,
      "disjoint preparation must not acquire pages between destinations: $prepared",
    )
  }

  @Test
  fun `offscreen-selection-is-not-implicit-demand`() {
    val required =
      requiredSurfacePages(
        pages = (0 until 10).map { page -> SurfacePageSpan(page, page * 100f, (page + 1) * 100f) },
        currentViewport = VerticalSpan(0f, 100f),
      )

    // There is intentionally no selection input that could add the offscreen page.
    assertEquals(setOf(0, 1), required)
    assertFalse(8 in required)
  }
}

private fun pages(vararg boundaries: Float): List<SurfacePageSpan> =
  boundaries.asList().zipWithNext().mapIndexed { page, (top, bottom) ->
    SurfacePageSpan(page, top, bottom)
  }
