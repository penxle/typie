package co.typie.platform

import androidx.compose.runtime.Composable

data class PickedFile(
  val bytes: ByteArray,
  val filename: String,
  val mimeType: String?,
  val imageWidth: Int? = null,
  val imageHeight: Int? = null,
)

enum class FilePickerSelectionMode {
  Single,
  Multiple,
}

sealed interface FilePickerResult {
  data object Cancelled : FilePickerResult

  data class Selected(val files: List<PickedFile>, val unreadableCount: Int = 0) :
    FilePickerResult {
    init {
      require(files.isNotEmpty())
      require(unreadableCount >= 0)
    }
  }

  data class Failed(val cause: Throwable) : FilePickerResult
}

@Composable
expect fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode = FilePickerSelectionMode.Single,
  onResult: (FilePickerResult) -> Unit,
): (mimeType: String) -> Unit

internal fun aggregateSelectedFiles(files: List<Result<PickedFile>>): FilePickerResult {
  val readableFiles = files.mapNotNull(Result<PickedFile>::getOrNull)
  if (readableFiles.isNotEmpty()) {
    return FilePickerResult.Selected(
      files = readableFiles,
      unreadableCount = files.size - readableFiles.size,
    )
  }

  return FilePickerResult.Failed(
    files.firstNotNullOfOrNull(Result<PickedFile>::exceptionOrNull)
      ?: IllegalStateException("File picker returned no selected files")
  )
}

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
