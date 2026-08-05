package co.typie.editor

internal data class SurfacePageSpan(val page: Int, val top: Float, val bottom: Float)

internal fun requiredSurfacePages(
  pages: List<SurfacePageSpan>,
  currentViewport: VerticalSpan? = null,
  activePages: Set<Int> = emptySet(),
  preparationViewports: List<VerticalSpan> = emptyList(),
): Set<Int> {
  if (pages.isEmpty()) return emptySet()

  val required = mutableSetOf<Int>()
  fun addViewport(viewport: VerticalSpan) {
    if (!viewport.hasFinitePositiveHeight()) return
    required += pages.intersecting(viewport.expandedBy(viewport.height))
    required +=
      pages.intersecting(viewport.expandedBy(viewport.height * 1.5f)).intersect(activePages)
  }
  currentViewport?.let(::addViewport)
  preparationViewports.forEach(::addViewport)

  return required
}

private fun List<SurfacePageSpan>.intersecting(viewport: VerticalSpan): Set<Int> = buildSet {
  this@intersecting.forEach { page ->
    if (page.hasFinitePositiveHeight() && page.intersects(viewport)) add(page.page)
  }
}

private fun VerticalSpan.expandedBy(distance: Float): VerticalSpan =
  VerticalSpan(top - distance, bottom + distance)

private fun VerticalSpan.hasFinitePositiveHeight(): Boolean =
  top.isFinite() && bottom.isFinite() && bottom > top

private fun SurfacePageSpan.hasFinitePositiveHeight(): Boolean =
  top.isFinite() && bottom.isFinite() && bottom > top

// Page and viewport spans are half-open: [top, bottom).
private fun SurfacePageSpan.intersects(viewport: VerticalSpan): Boolean =
  top < viewport.bottom && bottom > viewport.top
