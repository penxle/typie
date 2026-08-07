package co.typie.ui.component

import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation

private const val GOAL_DIGIT_LIMIT = 15
private const val GROUP_SIZE = 3
private const val GROUP_SEPARATOR = ','

fun String.filterGoalDigits(): String {
  val digits = filter { it in '0'..'9' }.take(GOAL_DIGIT_LIMIT)
  val normalized = digits.trimStart('0')

  return when {
    digits.isEmpty() -> ""
    normalized.isEmpty() -> "0"
    else -> normalized
  }
}

fun String.toGoalTargetOrNull(): Long? = toLongOrNull()?.takeIf { it > 0 }

private fun groupThousands(digits: String): String =
  digits.reversed().chunked(GROUP_SIZE).joinToString(GROUP_SEPARATOR.toString()).reversed()

private class CommaOffsetMapping(private val formatted: String) : OffsetMapping {
  override fun originalToTransformed(offset: Int): Int {
    var position = 0
    var seen = 0
    while (position < formatted.length && seen < offset) {
      if (formatted[position] != GROUP_SEPARATOR) {
        seen += 1
      }
      position += 1
    }

    return position
  }

  override fun transformedToOriginal(offset: Int): Int {
    val end = offset.coerceIn(0, formatted.length)
    var count = 0
    for (index in 0 until end) {
      if (formatted[index] != GROUP_SEPARATOR) {
        count += 1
      }
    }

    return count
  }
}

class CommaVisualTransformation : VisualTransformation {
  override fun filter(text: AnnotatedString): TransformedText {
    val formatted = groupThousands(text.text)

    return TransformedText(AnnotatedString(formatted), CommaOffsetMapping(formatted))
  }

  override fun equals(other: Any?): Boolean = other is CommaVisualTransformation

  override fun hashCode(): Int = 0
}
