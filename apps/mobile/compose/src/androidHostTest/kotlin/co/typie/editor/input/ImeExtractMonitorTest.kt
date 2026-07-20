package co.typie.editor.input

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ImeExtractMonitorTest {
  private fun extract(text: String, selectionStart: Int = 0, selectionEnd: Int = 0): ImeExtract =
    ImeExtract(text = text, selectionStart = selectionStart, selectionEnd = selectionEnd)

  @Test
  fun `unchanged text is not pushed again`() {
    val monitor = ImeExtractMonitor()

    assertTrue(monitor.shouldPushFor(extract("hello")))
    monitor.onExtractDelivered(extract("hello"))

    assertFalse(monitor.shouldPushFor(extract("hello")))
  }

  @Test
  fun `changed text is pushed`() {
    val monitor = ImeExtractMonitor()
    monitor.onExtractDelivered(extract("hello"))

    assertTrue(monitor.shouldPushFor(extract("hello!")))
  }

  @Test
  fun `a synchronous pull rebases the baseline`() {
    val monitor = ImeExtractMonitor()
    monitor.onExtractDelivered(extract("hello"))
    monitor.onExtractDelivered(extract("hello world"))

    assertFalse(monitor.shouldPushFor(extract("hello world")))
    assertTrue(monitor.shouldPushFor(extract("hello")))
  }

  @Test
  fun `selection mode transitions with unchanged text are pushed`() {
    val monitor = ImeExtractMonitor()

    // Collapsed -> range: FLAG_SELECTING changes, which only the extract conveys
    // (AOSP pushes on mSelectionModeChanged as well as mContentChanged).
    monitor.onExtractDelivered(extract("hello", selectionStart = 1, selectionEnd = 1))
    assertTrue(monitor.shouldPushFor(extract("hello", selectionStart = 1, selectionEnd = 4)))

    // Range -> collapsed.
    monitor.onExtractDelivered(extract("hello", selectionStart = 1, selectionEnd = 4))
    assertTrue(monitor.shouldPushFor(extract("hello", selectionStart = 4, selectionEnd = 4)))
  }

  @Test
  fun `selection moves within the same mode are not pushed`() {
    val monitor = ImeExtractMonitor()

    monitor.onExtractDelivered(extract("hello", selectionStart = 1, selectionEnd = 1))
    assertFalse(monitor.shouldPushFor(extract("hello", selectionStart = 3, selectionEnd = 3)))

    monitor.onExtractDelivered(extract("hello", selectionStart = 0, selectionEnd = 3))
    assertFalse(monitor.shouldPushFor(extract("hello", selectionStart = 1, selectionEnd = 4)))
  }
}
