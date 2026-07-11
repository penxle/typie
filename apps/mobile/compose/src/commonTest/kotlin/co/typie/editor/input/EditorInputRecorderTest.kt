package co.typie.editor.input

import kotlin.test.Test
import kotlin.test.assertEquals

class EditorInputRecorderTest {
  private fun entry(seq: Long, t: Long): RecordedInputEntry =
    RecordedInputEntry.Session(seq = seq, t = t, event = "start")

  @Test
  fun `assigns monotonically increasing seq starting at 1`() {
    val recorder = EditorInputRecorder()
    recorder.record { seq, t -> entry(seq, t) }
    recorder.record { seq, t -> entry(seq, t) }
    assertEquals(listOf(1L, 2L), recorder.snapshot().map { it.seq })
  }

  @Test
  fun `evicts oldest entries beyond capacity`() {
    val recorder = EditorInputRecorder()
    repeat(EditorInputRecorder.Capacity + 10) { recorder.record { seq, t -> entry(seq, t) } }
    val snapshot = recorder.snapshot()
    assertEquals(EditorInputRecorder.Capacity, snapshot.size)
    assertEquals(11L, snapshot.first().seq)
    assertEquals((EditorInputRecorder.Capacity + 10).toLong(), snapshot.last().seq)
  }

  @Test
  fun `snapshot does not clear the buffer`() {
    val recorder = EditorInputRecorder()
    recorder.record { seq, t -> entry(seq, t) }
    recorder.snapshot()
    assertEquals(1, recorder.snapshot().size)
  }
}
