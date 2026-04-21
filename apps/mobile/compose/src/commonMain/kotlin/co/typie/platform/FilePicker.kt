package co.typie.platform

import androidx.compose.runtime.Composable

data class PlatformFile(val bytes: ByteArray, val filename: String, val mimeType: String?)

enum class FilePickerSelectionMode {
  Single,
  Multiple,
}

@Composable
expect fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode = FilePickerSelectionMode.Single,
  onResult: (List<PlatformFile>) -> Unit,
): (mimeType: String) -> Unit

internal fun pickedFilename(originalFilename: String?, mimeType: String?): String {
  val trimmedFilename = originalFilename?.trim()?.takeIf { it.isNotEmpty() }
  if (trimmedFilename != null) {
    return trimmedFilename
  }

  return when (mimeType?.substringBefore(';')?.lowercase()) {
    "image/jpeg",
    "image/jpg" -> "image.jpg"
    "image/png" -> "image.png"
    "image/webp" -> "image.webp"
    "image/heic" -> "image.heic"
    "image/heif" -> "image.heif"
    else -> "file"
  }
}
