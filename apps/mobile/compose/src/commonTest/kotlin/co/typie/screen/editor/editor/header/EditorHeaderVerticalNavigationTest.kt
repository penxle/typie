package co.typie.screen.editor.editor.header

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class EditorHeaderVerticalNavigationTest {
  @Test
  fun `same visual line permits exit when collapsed selection is unchanged`() {
    val value = value(selection = TextRange(3))
    val probe = probe(value = value, visualLine = 2)

    assertTrue(shouldExit(probe = probe, currentValue = value, currentVisualLine = 2))
  }

  @Test
  fun `same visual line permits exit when native handling changes collapsed selection`() {
    val value = value(selection = TextRange(3))
    val probe = probe(value = value, visualLine = 2)

    assertTrue(
      shouldExit(
        probe = probe,
        currentValue = value(selection = TextRange(6)),
        currentVisualLine = 2,
      )
    )
  }

  @Test
  fun `different visual line cancels exit even when selection reaches text end`() {
    val value = value(selection = TextRange(3))
    val probe = probe(value = value, visualLine = 1)

    assertFalse(
      shouldExit(
        probe = probe,
        currentValue = value(selection = TextRange(6)),
        currentVisualLine = 2,
      )
    )
  }

  @Test
  fun `changed text or missing line cancels exit`() {
    val value = value(selection = TextRange(3))
    val probe = probe(value = value, visualLine = 2)

    assertFalse(
      shouldExit(
        probe = probe,
        currentValue = value(text = "changed", selection = TextRange(3)),
        currentVisualLine = 2,
      )
    )
    assertFalse(shouldExit(probe = probe, currentValue = value, currentVisualLine = null))
  }

  @Test
  fun `composition before or after native handling cancels exit`() {
    val composing = value(selection = TextRange(3), composition = TextRange(1, 3))
    val plain = value(selection = TextRange(3))

    assertFalse(shouldExit(probe = probe(value = composing), currentValue = plain))
    assertFalse(shouldExit(probe = probe(value = plain), currentValue = composing))
  }

  @Test
  fun `range selection before or after native handling cancels exit`() {
    val range = value(selection = TextRange(1, 3))
    val collapsed = value(selection = TextRange(3))

    assertFalse(shouldExit(probe = probe(value = range), currentValue = collapsed))
    assertFalse(shouldExit(probe = probe(value = collapsed), currentValue = range))
  }

  @Test
  fun `lost focus disablement or stale token cancels exit`() {
    val value = value(selection = TextRange(3))
    val probe = probe(value = value)

    assertFalse(shouldExit(probe = probe, currentValue = value, focused = false))
    assertFalse(shouldExit(probe = probe, currentValue = value, enabled = false))
    assertFalse(shouldExit(probe = probe, currentValue = value, latestToken = 8))
  }

  private fun probe(value: TextFieldValue, visualLine: Int = 1): HeaderVerticalExitProbe =
    HeaderVerticalExitProbe(token = 7, value = value, visualLine = visualLine)

  private fun shouldExit(
    probe: HeaderVerticalExitProbe,
    currentValue: TextFieldValue,
    focused: Boolean = true,
    enabled: Boolean = true,
    latestToken: Long = 7,
    currentVisualLine: Int? = probe.visualLine,
  ): Boolean =
    shouldExitHeaderFieldAfterNativeVerticalMove(
      probe = probe,
      currentValue = currentValue,
      focused = focused,
      enabled = enabled,
      latestToken = latestToken,
      currentVisualLine = currentVisualLine,
    )

  private fun value(
    text: String = "header",
    selection: TextRange,
    composition: TextRange? = null,
  ): TextFieldValue = TextFieldValue(text = text, selection = selection, composition = composition)
}
