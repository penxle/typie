package co.typie.screen.editor.editor.header

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import kotlin.test.Test
import kotlin.test.assertEquals

class EditorHeaderInputRulesTest {
  @Test
  fun `title sanitizing removes line breaks and enforces the shared max length`() {
    val value = "제목\r\n줄\n바꿈" + "a".repeat(120)
    val expectedPrefix = "제목 줄 바꿈"

    val sanitized = sanitizeTitleInput(value)

    assertEquals(EditorTitleMaxLength, sanitized.length)
    assertEquals(expectedPrefix, sanitized.take(expectedPrefix.length))
  }

  @Test
  fun `subtitle field sanitizing clamps selection after truncation`() {
    val value = TextFieldValue(text = "a".repeat(120), selection = TextRange(start = 96, end = 120))

    val sanitized = sanitizeSubtitleFieldValue(value)

    assertEquals(100, sanitized.text.length)
    assertEquals(TextRange(start = 96, end = 100), sanitized.selection)
  }
}
