package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import java.awt.FileDialog
import java.awt.Frame
import java.awt.image.BufferedImage
import java.io.File
import java.nio.file.Files
import javax.imageio.ImageIO
import kotlinx.io.asSource
import kotlinx.io.buffered

@Composable
actual fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode,
  onResult: (FilePickerResult) -> Unit,
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
      val selectedFiles = dialog.files
      currentOnResult.value(
        if (selectedFiles.isEmpty()) {
          FilePickerResult.Cancelled
        } else {
          aggregateSelectedFiles(
            selectedFiles.map { file ->
              runCatching {
                val image =
                  when (contentType) {
                    "image" -> file.decodeImageOrNull()
                    else -> null
                  }
                PickedFile(
                  filename = file.name,
                  mimeType = file.probeContentType(),
                  size = file.length(),
                  previewModel = file,
                  imageWidth = image?.width,
                  imageHeight = image?.height,
                  openSource = { file.inputStream().asSource().buffered() },
                )
              }
            }
          )
        }
      )
    }
  }
}

internal fun File.probeContentType(): String? {
  return runCatching { Files.probeContentType(toPath()) }.getOrNull()
}

internal fun File.decodeImageOrNull(): BufferedImage? =
  runCatching { ImageIO.read(this) }.getOrNull()
