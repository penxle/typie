package co.typie.media

import androidx.compose.runtime.Composable

data class PickedImage(
  val bytes: ByteArray,
  val filename: String,
  val mimeType: String?,
)

val imagePickerDialogTitle = "이미지 선택"

fun pickedImageFilename(
  originalFilename: String?,
  mimeType: String?,
): String {
  val trimmedFilename = originalFilename?.trim()?.takeIf { it.isNotEmpty() }
  if (trimmedFilename != null) {
    return trimmedFilename
  }

  return when (mimeType?.substringBefore(';')?.lowercase()) {
    "image/jpeg", "image/jpg" -> "image.jpg"
    "image/png" -> "image.png"
    "image/webp" -> "image.webp"
    "image/heic" -> "image.heic"
    "image/heif" -> "image.heif"
    else -> "image"
  }
}

@Composable
expect fun rememberImagePicker(
  onResult: (PickedImage?) -> Unit,
): () -> Unit
