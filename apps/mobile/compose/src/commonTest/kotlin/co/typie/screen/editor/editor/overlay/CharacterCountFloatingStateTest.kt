package co.typie.screen.editor.editor.overlay

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class CharacterCountFloatingStateTest {
  @Test
  fun restoresAbsolutePositionFromRelative() {
    val state =
      CharacterCountFloatingState(
        relativeX = 0.5f,
        relativeY = 0.25f,
        persist = { _, _ -> },
      )

    state.onViewportMeasured(width = 400f, height = 800f, widgetWidth = 100f, widgetHeight = 40f)

    // relative 0.5 of free width (400 - 100 = 300) = 150; 0.25 of free height (800 - 40 = 760) =
    // 190
    assertEquals(150f, state.offsetX)
    assertEquals(190f, state.offsetY)
  }

  @Test
  fun clampsWithinViewport() {
    val state = CharacterCountFloatingState(relativeX = 0f, relativeY = 0f, persist = { _, _ -> })
    state.onViewportMeasured(width = 400f, height = 800f, widgetWidth = 100f, widgetHeight = 40f)

    // drag far beyond bottom-right; offset must clamp to max free space
    state.onDrag(dx = 9999f, dy = 9999f)

    assertEquals(300f, state.offsetX) // 400 - 100
    assertEquals(760f, state.offsetY) // 800 - 40
  }

  @Test
  fun clampNeverGoesNegative() {
    val state =
      CharacterCountFloatingState(relativeX = 0.1f, relativeY = 0.1f, persist = { _, _ -> })
    state.onViewportMeasured(width = 400f, height = 800f, widgetWidth = 100f, widgetHeight = 40f)

    state.onDrag(dx = -9999f, dy = -9999f)

    assertEquals(0f, state.offsetX)
    assertEquals(0f, state.offsetY)
  }

  @Test
  fun clampRespectsBottomOcclusion() {
    val state = CharacterCountFloatingState(relativeX = 0f, relativeY = 0f, persist = { _, _ -> })
    // 200px of the bottom is occluded by toolbar/keyboard
    state.onViewportMeasured(
      width = 400f,
      height = 800f,
      widgetWidth = 100f,
      widgetHeight = 40f,
      bottomOcclusion = 200f,
    )

    state.onDrag(dx = 0f, dy = 9999f)

    // max y = height - bottomOcclusion - widgetHeight = 800 - 200 - 40 = 560
    assertEquals(560f, state.offsetY)
  }

  @Test
  fun clampRespectsTopOcclusion() {
    val state = CharacterCountFloatingState(relativeX = 0f, relativeY = 0f, persist = { _, _ -> })
    // 120px of the top is occluded by the header/safe area
    state.onViewportMeasured(
      width = 400f,
      height = 800f,
      widgetWidth = 100f,
      widgetHeight = 40f,
      topOcclusion = 120f,
    )

    // relativeY 0 must resolve to the top of the free area, i.e. below the occluded header
    assertEquals(120f, state.offsetY)

    // dragging above the header must clamp to the top occlusion, never over the header
    state.onDrag(dx = 0f, dy = -9999f)
    assertEquals(120f, state.offsetY)

    // dragging far down clamps to top + free height = 120 + (800 - 120 - 40) = height -
    // widgetHeight = 760
    state.onDrag(dx = 0f, dy = 9999f)
    assertEquals(760f, state.offsetY)
  }

  @Test
  fun topAndBottomOcclusionShrinkFreeHeightTogether() {
    var savedY: Float? = null
    val state =
      CharacterCountFloatingState(relativeX = 0f, relativeY = 0f, persist = { _, y -> savedY = y })
    state.onViewportMeasured(
      width = 400f,
      height = 800f,
      widgetWidth = 100f,
      widgetHeight = 40f,
      topOcclusion = 100f,
      bottomOcclusion = 100f,
    )

    // free height = 800 - 100 - 100 - 40 = 560; drag to the middle of it
    state.onDrag(dx = 0f, dy = 280f)
    state.onDragEnd()

    // offset measured from the top occlusion: 100 + 280 = 380
    assertEquals(380f, state.offsetY)
    // persisted relative is fraction of the free height, occlusion-independent: 280 / 560 = 0.5
    assertEquals(0.5f, savedY)
  }

  @Test
  fun dragEndPersistsRelativePosition() {
    var savedX: Float? = null
    var savedY: Float? = null
    val state =
      CharacterCountFloatingState(
        relativeX = 0f,
        relativeY = 0f,
        persist = { x, y ->
          savedX = x
          savedY = y
        },
      )
    state.onViewportMeasured(width = 400f, height = 800f, widgetWidth = 100f, widgetHeight = 40f)

    state.onDrag(dx = 150f, dy = 190f)
    state.onDragEnd()

    // 150 / (400 - 100) = 0.5 ; 190 / (800 - 40) = 0.25
    assertEquals(0.5f, savedX)
    assertEquals(0.25f, savedY)
  }

  @Test
  fun viewportChangeDuringDragKeepsDraggedPosition() {
    val state = CharacterCountFloatingState(relativeX = 0f, relativeY = 0f, persist = { _, _ -> })
    state.onViewportMeasured(width = 400f, height = 800f, widgetWidth = 100f, widgetHeight = 40f)

    state.onDragStart()
    state.onDrag(dx = 200f, dy = 300f)

    // the keyboard opens mid-drag: re-measuring must not reset the dragged offset from the stale
    // relative fraction...
    state.onViewportMeasured(
      width = 400f,
      height = 800f,
      widgetWidth = 100f,
      widgetHeight = 40f,
      bottomOcclusion = 200f,
    )
    assertEquals(200f, state.offsetX)
    assertEquals(300f, state.offsetY)

    // ...but further drags clamp against the new bounds (800 - 200 - 40 = 560)
    state.onDrag(dx = 0f, dy = 9999f)
    assertEquals(560f, state.offsetY)

    // drag end persists against the new free space: 560 / 560 = 1.0
    var savedY: Float? = null
    val persistingState =
      CharacterCountFloatingState(relativeX = 0f, relativeY = 0f, persist = { _, y -> savedY = y })
    persistingState.onViewportMeasured(
      width = 400f,
      height = 800f,
      widgetWidth = 100f,
      widgetHeight = 40f,
    )
    persistingState.onDragStart()
    persistingState.onDrag(dx = 0f, dy = 9999f)
    persistingState.onDragEnd()
    assertEquals(1f, savedY)

    // after the drag ends, re-measuring resolves from the persisted relative position again
    persistingState.onViewportMeasured(
      width = 400f,
      height = 800f,
      widgetWidth = 100f,
      widgetHeight = 40f,
    )
    assertEquals(760f, persistingState.offsetY)
  }

  @Test
  fun toggleExpands() {
    val state = CharacterCountFloatingState(relativeX = 0f, relativeY = 0f, persist = { _, _ -> })

    assertFalse(state.expanded)
    state.toggleExpanded()
    assertTrue(state.expanded)
    state.toggleExpanded()
    assertFalse(state.expanded)
  }
}
