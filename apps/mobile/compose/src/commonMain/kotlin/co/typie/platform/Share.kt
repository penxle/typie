package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInWindow
import androidx.compose.ui.platform.LocalDensity

interface Share {
  suspend fun share(
    bytes: ByteArray,
    filename: String,
    mimeType: String,
    anchor: ShareAnchor?,
  ): Boolean

  suspend fun share(text: String, anchor: ShareAnchor?): Boolean
}

private const val MAX_SHARE_FILENAME_BYTES = 255

internal fun sanitizeShareFilename(filename: String): String {
  val sanitized =
    filename
      .map { if (it == '/' || it == '\\' || it.isISOControl()) '_' else it }
      .joinToString("")
      .trim()

  val usable = if (sanitized.isEmpty() || sanitized.all { it == '.' }) "file" else sanitized

  return truncateShareFilename(usable)
}

private fun truncateShareFilename(filename: String): String {
  if (filename.utf8Size() <= MAX_SHARE_FILENAME_BYTES) return filename

  val dotIndex = filename.lastIndexOf('.')
  val extension = if (dotIndex > 0) filename.substring(dotIndex) else ""
  val stem = if (dotIndex > 0) filename.substring(0, dotIndex) else filename
  val stemBudget = MAX_SHARE_FILENAME_BYTES - extension.utf8Size()
  val truncatedStem = if (stemBudget > 0) stem.takeUtf8Bytes(stemBudget).trimEnd() else ""

  return if (truncatedStem.isEmpty()) {
    filename.takeUtf8Bytes(MAX_SHARE_FILENAME_BYTES)
  } else {
    truncatedStem + extension
  }
}

private fun String.utf8Size(): Int = encodeToByteArray().size

private fun String.takeUtf8Bytes(limit: Int): String {
  var bytes = 0
  var index = 0
  while (index < length) {
    val paired =
      this[index].isHighSurrogate() && index + 1 < length && this[index + 1].isLowSurrogate()
    val size = if (paired) 4 else utf8SizeOf(this[index])
    if (bytes + size > limit) break
    bytes += size
    index += if (paired) 2 else 1
  }

  return substring(0, index)
}

private fun utf8SizeOf(char: Char): Int =
  when {
    char.code < 0x80 -> 1
    char.code < 0x800 -> 2
    else -> 3
  }

data class ShareAnchor(val x: Double, val y: Double, val width: Double, val height: Double)

class ShareAnchorState internal constructor() {
  internal var density = 1f

  var value: ShareAnchor? = null
    private set

  val modifier: Modifier = Modifier.onGloballyPositioned { coordinates ->
    val position = coordinates.positionInWindow()
    val size = coordinates.size
    value =
      ShareAnchor(
        x = (position.x / density).toDouble(),
        y = (position.y / density).toDouble(),
        width = (size.width / density).toDouble(),
        height = (size.height / density).toDouble(),
      )
  }
}

@Composable
fun rememberShareAnchor(): ShareAnchorState {
  val state = remember { ShareAnchorState() }
  state.density = LocalDensity.current.density
  return state
}
