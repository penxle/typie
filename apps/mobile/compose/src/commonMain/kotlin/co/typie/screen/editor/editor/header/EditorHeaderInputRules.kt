package co.typie.screen.editor.editor.header

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue

internal const val EditorTitleMaxLength = 100

internal fun sanitizeTitleInput(text: String): String = sanitizeTitleLikeInput(text)

internal fun sanitizeSubtitleInput(text: String): String = sanitizeTitleLikeInput(text)

internal fun sanitizeTitleFieldValue(value: TextFieldValue): TextFieldValue =
  sanitizeTitleLikeFieldValue(value)

internal fun sanitizeSubtitleFieldValue(value: TextFieldValue): TextFieldValue =
  sanitizeTitleLikeFieldValue(value)

private fun sanitizeTitleLikeInput(text: String): String {
  return text
    .replace("\r\n", "\n")
    .replace('\r', '\n')
    .replace('\n', ' ')
    .take(EditorTitleMaxLength)
}

private fun sanitizeTitleLikeFieldValue(value: TextFieldValue): TextFieldValue {
  val sanitized = sanitizeTitleLikeInput(value.text)
  if (sanitized == value.text) {
    return value
  }

  return value.copy(
    text = sanitized,
    selection =
      TextRange(
        start = value.selection.start.coerceIn(0, sanitized.length),
        end = value.selection.end.coerceIn(0, sanitized.length),
      ),
    composition =
      value.composition?.let {
        TextRange(
          start = it.start.coerceIn(0, sanitized.length),
          end = it.end.coerceIn(0, sanitized.length),
        )
      },
  )
}
