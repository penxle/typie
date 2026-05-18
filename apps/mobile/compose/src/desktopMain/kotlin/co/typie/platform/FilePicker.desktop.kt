package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import java.awt.FileDialog
import java.awt.Frame
import java.awt.image.BufferedImage
import java.io.ByteArrayInputStream
import java.io.File
import java.nio.file.Files
import javax.imageio.ImageIO

@Composable
actual fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode,
  onResult: (List<PickedFile>) -> Unit,
): (mimeType: String) -> Unit {
  val currentOnResult = rememberUpdatedState(onResult)

  return remember(selectionMode) {
    { mimeType: String ->
      val contentType = mimeType.substringBefore('/')
      val title =
        when (contentType) {
          "image" -> "이미지 선택"
          else -> "파일 선택"
        }
      val dialog =
        FileDialog(null as Frame?, title, FileDialog.LOAD).apply {
          when (contentType) {
            "image" -> {
              setFilenameFilter { _, name ->
                val lower = name.lowercase()
                lower.endsWith(".png") ||
                  lower.endsWith(".jpg") ||
                  lower.endsWith(".jpeg") ||
                  lower.endsWith(".webp") ||
                  lower.endsWith(".heic")
              }
            }
          }
          isMultipleMode = selectionMode == FilePickerSelectionMode.Multiple
          isVisible = true
        }
      val files =
        dialog.files.map { file ->
          val bytes = file.readBytes()
          val image =
            when (contentType) {
              "image" -> bytes.decodeImageOrNull()
              else -> null
            }
          PickedFile(
            bytes = bytes,
            filename = file.name,
            mimeType = file.probeContentType(),
            imageWidth = image?.width,
            imageHeight = image?.height,
          )
        }

      currentOnResult.value(files)
    }
  }
}

private fun File.probeContentType(): String? {
  return runCatching { Files.probeContentType(toPath()) }.getOrNull()
}

private fun ByteArray.decodeImageOrNull(): BufferedImage? =
  runCatching { ImageIO.read(ByteArrayInputStream(this)) }.getOrNull()
