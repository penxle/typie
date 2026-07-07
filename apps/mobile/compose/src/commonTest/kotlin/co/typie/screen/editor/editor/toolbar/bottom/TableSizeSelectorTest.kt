package co.typie.screen.editor.editor.toolbar.bottom

import kotlin.test.Test
import kotlin.test.assertEquals

class TableSizeSelectorTest {
  @Test
  fun gridOffsetSelectsClampedTableSize() {
    assertEquals(
      TableSizeSelection(rows = 1, cols = 1),
      tableSizeSelectionFromGridOffset(
        x = -20f,
        y = -20f,
        cellSize = GridCellSize,
        gap = GridGap,
        outerPadding = GridOuterPadding,
      ),
    )
    assertEquals(
      TableSizeSelection(rows = 5, cols = 3),
      tableSizeSelectionFromGridOffset(
        x = GridOuterPadding + 2 * GridStride + 1f,
        y = GridOuterPadding + 4 * GridStride + 1f,
        cellSize = GridCellSize,
        gap = GridGap,
        outerPadding = GridOuterPadding,
      ),
    )
    assertEquals(
      TableSizeSelection(rows = 10, cols = 10),
      tableSizeSelectionFromGridOffset(
        x = 10_000f,
        y = 10_000f,
        cellSize = GridCellSize,
        gap = GridGap,
        outerPadding = GridOuterPadding,
      ),
    )
  }

  @Test
  fun centeredScrollTargetKeepsSelectedCellInRange() {
    assertEquals(
      0,
      tableSizeCenteredScrollTarget(
        index = 0,
        viewportExtent = 220f,
        maxScroll = 184,
        cellSize = GridCellSize,
        gap = GridGap,
        outerPadding = GridOuterPadding,
      ),
    )
    assertEquals(
      112,
      tableSizeCenteredScrollTarget(
        index = 5,
        viewportExtent = 220f,
        maxScroll = 184,
        cellSize = GridCellSize,
        gap = GridGap,
        outerPadding = GridOuterPadding,
      ),
    )
    assertEquals(
      184,
      tableSizeCenteredScrollTarget(
        index = 9,
        viewportExtent = 220f,
        maxScroll = 184,
        cellSize = GridCellSize,
        gap = GridGap,
        outerPadding = GridOuterPadding,
      ),
    )
  }
}

private const val GridCellSize = 36f
private const val GridGap = 4f
private const val GridOuterPadding = GridGap
private const val GridStride = GridCellSize + GridGap
