package co.typie.editor.gesture

import androidx.compose.ui.geometry.Offset
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class EditorInteractionCoreTest {
  @Test
  fun `interaction core keeps dnd ahead of pinch`() {
    val core = EditorInteractionCore()
    val dnd = core.reduce(EditorInteractionMode.Idle, EditorInteractionEvent.dndStart(local = true))

    assertEquals(EditorInteractionMode.DndLocal, dnd)
    assertEquals(
      EditorInteractionMode.DndLocal,
      core.reduce(dnd, EditorInteractionEvent.PinchStart),
    )
  }

  @Test
  fun `interaction core blocks tap dispatch on invalid page or pinch`() {
    val core = EditorInteractionCore()

    assertEquals(
      EditorInteractionBlockReason.PageOutOfRange,
      core.blockReason(
        command = EditorInteractionCommand.TapDispatch(page = -1),
        runtime = EditorInteractionRuntimeRead(),
      ),
    )
    assertEquals(
      EditorInteractionBlockReason.Pinching,
      core.blockReason(
        command = EditorInteractionCommand.TapDispatch(page = 0),
        runtime = EditorInteractionRuntimeRead(pinchIsPinching = true),
      ),
    )
    assertTrue(
      core.decide(
        command = EditorInteractionCommand.TapDispatch(page = 0),
        runtime = EditorInteractionRuntimeRead(),
      )
    )
  }

  @Test
  fun `interaction core blocks raw pan without a single tracked pointer`() {
    val core = EditorInteractionCore()

    assertEquals(
      EditorInteractionBlockReason.NonSinglePointer,
      core.blockReason(
        command = EditorInteractionCommand.PanApplyRaw(hasPreviousPointerPosition = true),
        runtime = EditorInteractionRuntimeRead(pinchPointerCount = 2),
      ),
    )
    assertEquals(
      EditorInteractionBlockReason.PointerTrackMissing,
      core.blockReason(
        command = EditorInteractionCommand.PanApplyRaw(hasPreviousPointerPosition = false),
        runtime = EditorInteractionRuntimeRead(pinchPointerCount = 1),
      ),
    )
    assertTrue(
      core.decide(
        command = EditorInteractionCommand.PanApplyRaw(hasPreviousPointerPosition = true),
        runtime = EditorInteractionRuntimeRead(pinchPointerCount = 1),
      )
    )
  }

  @Test
  fun `interaction core blocks long press when selection sessions own the pointer`() {
    val core = EditorInteractionCore()

    assertEquals(
      EditorInteractionBlockReason.ViewportUnavailable,
      core.blockReason(
        command = EditorInteractionCommand.LongPressStart(viewportPosition = null),
        runtime = EditorInteractionRuntimeRead(),
      ),
    )
    assertEquals(
      EditorInteractionBlockReason.DoubleTapSelecting,
      core.blockReason(
        command = EditorInteractionCommand.LongPressStart(viewportPosition = Offset.Zero),
        runtime = EditorInteractionRuntimeRead(doubleTapActive = true),
      ),
    )
    assertTrue(
      core.decide(
        command = EditorInteractionCommand.LongPressStart(viewportPosition = Offset.Zero),
        runtime = EditorInteractionRuntimeRead(),
      )
    )
  }

  @Test
  fun `interaction core blocks selection handle updates outside handle dragging mode`() {
    val core = EditorInteractionCore()

    assertEquals(
      EditorInteractionBlockReason.NotActive,
      core.blockReason(
        command = EditorInteractionCommand.SelectionHandleUpdate,
        runtime = EditorInteractionRuntimeRead(),
      ),
    )
    assertTrue(
      core.decide(
        command = EditorInteractionCommand.SelectionHandleUpdate,
        runtime = EditorInteractionRuntimeRead(mode = EditorInteractionMode.SelectionHandleDragging),
      )
    )
  }
}
