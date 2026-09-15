package co.typie.editor.input

import androidx.compose.ui.geometry.Offset
import co.typie.editor.EditorViewportTransform
import co.typie.editor.PagePoint
import co.typie.editor.ffi.Affinity
import co.typie.editor.ffi.CaretMetrics
import co.typie.editor.ffi.CursorMetrics
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.PageRect
import co.typie.editor.ffi.Position
import co.typie.editor.ffi.Rect
import co.typie.editor.ffi.SelectionEndpoints
import co.typie.editor.ffi.SelectionOp
import co.typie.editor.ffi.Size
import kotlin.math.floor
import kotlin.math.round
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class EditorFloatingCursorTest {
  private val endpoints =
    SelectionEndpoints(
      from = PageRect(pageIdx = 0, rect = Rect(x = 40f, y = 40f, width = 0f, height = 20f)),
      to = PageRect(pageIdx = 1, rect = Rect(x = 140f, y = 60f, width = 0f, height = 20f)),
      fromPosition = Position(node = "paragraph", offset = 2, affinity = Affinity.Downstream),
      toPosition = Position(node = "paragraph", offset = 20, affinity = Affinity.Upstream),
    )
  private val transform =
    EditorViewportTransform(
      pageOffsets = mapOf(0 to Offset.Zero, 1 to Offset(0f, 220f)),
      pageSizes = List(2) { Size(width = 200f, height = 200f) },
    )

  @Test
  fun `paragraph break handle geometry does not trigger a caret move`() {
    val paragraphBreakEndpoints =
      endpoints.copy(to = PageRect(pageIdx = 0, rect = Rect(200f, 40f, 0f, 20f)))
    val session =
      beginRange(selectionEndpoints = paragraphBreakEndpoints) { point ->
        val cursor = cursorAtPoint(point)
        if (point.page == 0)
          cursor.copy(caret = cursor.caret.copy(x = cursor.caret.x.coerceAtMost(64f)))
        else cursor
      }

    assertNull(session.update(0.5f, 0f, transform))
    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 50f, y = 50f))),
      session.update(10f, 0f, transform),
    )
  }

  @Test
  fun `paragraph break selection moves left and up from the visible end handle`() {
    val paragraphBreakEndpoints =
      endpoints.copy(to = PageRect(pageIdx = 0, rect = Rect(200f, 40f, 0f, 20f)))
    fun session() =
      beginRange(selectionEndpoints = paragraphBreakEndpoints) { point ->
        val cursor = cursorAtPoint(point)
        if (point.page == 0)
          cursor.copy(caret = cursor.caret.copy(x = cursor.caret.x.coerceIn(40f, 64f)))
        else cursor
      }

    val left = session()
    // The blank area after abc still resolves to the same caret. Entering the text moves it.
    assertNull(left.update(-138f, 0f, transform))
    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 58f, y = 50f))),
      left.update(-142f, 0f, transform),
    )
    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 60f, y = 50f))),
      left.update(-140f, 0f, transform),
    )
    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 200f, y = 38f))),
      session().update(0f, -12f, transform),
    )
  }

  @Test
  fun `range starts from the opposite endpoint of the first caret movement`() {
    val movements =
      listOf(
        Offset(-10f, 0f) to true,
        Offset(10f, 0f) to false,
        Offset(0f, -12f) to true,
        Offset(0f, 12f) to false,
        Offset(-10f, 3f) to true,
        Offset(10f, -3f) to false,
        Offset(3f, -12f) to true,
        Offset(-3f, 12f) to false,
      )

    movements.forEach { (movement, fromEnd) ->
      val session = beginRange()

      assertEquals(
        listOf(
          Message.Selection(
            SelectionOp.SetAt(
              page = if (fromEnd) 1 else 0,
              x = (if (fromEnd) 140f else 40f) + movement.x,
              y = (if (fromEnd) 70f else 50f) + movement.y,
            )
          )
        ),
        session.update(movement.x, movement.y, transform),
        "First movement: $movement",
      )
    }
  }

  @Test
  fun `range stays selected while hit testing resolves to the same caret`() {
    val session = beginRange()

    assertNull(session.update(0f, 0f, transform))
    assertNull(session.update(-0.5f, -0.5f, transform))
    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 50f, y = 50f))),
      session.update(10f, 0f, transform),
    )
  }

  @Test
  fun `caret direction takes precedence over the dominant raw delta`() {
    val session = beginRange()

    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 44.66f, y = 44.77f))),
      session.update(4.66f, -5.23f, transform),
    )
  }

  @Test
  fun `initial upward drift does not lock a rightward drag to the end`() {
    val session = beginRange()

    // Initial UIKit updates captured on iPhone 16 during a rightward spacebar drag.
    val initialUpdates =
      listOf(
        Offset(0f, -5.23f),
        Offset(1.04f, -4.88f),
        Offset(2.10f, -4.18f),
        Offset(3.98f, -3.43f),
      )
    initialUpdates.forEach { assertNull(session.update(it.x, it.y, transform)) }

    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 44.66f, y = 46.57f))),
      session.update(4.66f, -3.43f, transform),
    )
  }

  @Test
  fun `reversing direction keeps the first endpoint as the origin`() {
    val session = beginRange()

    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 1, x = 130f, y = 70f))),
      session.update(-10f, 0f, transform),
    )
    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 1, x = 150f, y = 80f))),
      session.update(10f, 10f, transform),
    )
  }

  @Test
  fun `a new gesture chooses its own origin`() {
    val first = beginRange()
    first.update(-10f, 0f, transform)

    val next = beginRange()
    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 50f, y = 50f))),
      next.update(10f, 0f, transform),
    )
  }

  @Test
  fun `unavailable geometry preserves the range`() {
    var geometryAvailable = true
    val session = beginRange { point -> if (geometryAvailable) cursorAtPoint(point) else null }
    geometryAvailable = false

    assertNull(session.update(-0.5f, 0f, transform))
    assertNull(session.update(-20f, 20f, transform))
  }

  @Test
  fun `collapsed selection moves immediately from the current cursor`() {
    val session =
      assertNotNull(
        EditorFloatingCursorSession.begin(
          cursor =
            CursorMetrics(
              pageIdx = 0,
              caret = CaretMetrics(x = 40f, y = 42f, height = 16f),
              line = Rect(x = 0f, y = 40f, width = 200f, height = 20f),
            ),
          selectionEndpoints = null,
          cursorAt = ::cursorAtPoint,
        )
      )

    assertEquals(
      listOf(Message.Selection(SelectionOp.SetAt(page = 0, x = 40.5f, y = 40f))),
      session.update(0.5f, 0f, transform),
    )
  }

  @Test
  fun `floating cursor delta is mapped through viewport zoom`() {
    val point =
      resolveFloatingCursorPoint(
        origin = PagePoint(page = 0, x = 10f, y = 20f),
        dx = 20f,
        dy = 10f,
        transform =
          EditorViewportTransform(
            pageOffsets = mapOf(0 to Offset.Zero),
            pageSizes = listOf(Size(width = 100f, height = 100f)),
            displayZoom = 2f,
          ),
      )

    assertEquals(0, point?.page)
    assertEquals(20f, point?.x)
    assertEquals(25f, point?.y)
  }

  @Test
  fun `floating cursor can cross page boundary`() {
    val point =
      resolveFloatingCursorPoint(
        origin = PagePoint(page = 0, x = 10f, y = 90f),
        dx = 0f,
        dy = 30f,
        transform =
          EditorViewportTransform(
            pageOffsets = mapOf(0 to Offset(0f, 0f), 1 to Offset(0f, 120f)),
            pageSizes =
              listOf(Size(width = 100f, height = 100f), Size(width = 100f, height = 100f)),
            displayZoom = 1f,
          ),
      )

    assertEquals(1, point?.page)
    assertEquals(10f, point?.x)
    assertEquals(0f, point?.y)
  }

  @Test
  fun `floating cursor returns null when origin page is missing`() {
    val point =
      resolveFloatingCursorPoint(
        origin = PagePoint(page = 1, x = 10f, y = 20f),
        dx = 0f,
        dy = 0f,
        transform =
          EditorViewportTransform(
            pageOffsets = mapOf(0 to Offset.Zero),
            pageSizes = listOf(Size(width = 100f, height = 100f)),
          ),
      )

    assertNull(point)
  }

  private fun beginRange(
    selectionEndpoints: SelectionEndpoints = endpoints,
    cursorAt: (PagePoint) -> CursorMetrics? = ::cursorAtPoint,
  ): EditorFloatingCursorSession =
    assertNotNull(
      EditorFloatingCursorSession.begin(
        cursor = null,
        selectionEndpoints = selectionEndpoints,
        cursorAt = cursorAt,
      )
    )

  // Fixed-width glyphs and lines; actual layout hit testing is covered in editor-core.
  private fun cursorAtPoint(point: PagePoint): CursorMetrics {
    val originX = if (point.page == 0) 40f else 140f
    val x = originX + round((point.x - originX) / 8f) * 8f
    val lineY = floor(point.y / 20f) * 20f
    return CursorMetrics(
      pageIdx = point.page,
      caret = CaretMetrics(x = x, y = lineY, height = 20f),
      line = Rect(x = 0f, y = lineY, width = 200f, height = 20f),
    )
  }
}
