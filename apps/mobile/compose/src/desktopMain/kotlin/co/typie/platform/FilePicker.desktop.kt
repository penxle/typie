package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import java.awt.FileDialog
import java.awt.Frame
import java.awt.image.BufferedImage
import java.io.File
import java.nio.file.Files
import javax.imageio.ImageIO
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.io.asSource
import kotlinx.io.buffered

@Composable
actual fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode,
  onResult: (FilePickerResult) -> Unit,
): (mimeType: String) -> Unit {
  val scope = rememberCoroutineScope()
  val currentOnResult = rememberUpdatedState(onResult)

  return remember(selectionMode, scope) {
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
                  lower.endsWith(".heic") ||
                  lower.endsWith(".svg")
              }
            }
          }
          isMultipleMode = selectionMode == FilePickerSelectionMode.Multiple
          isVisible = true
        }
      val selectedFiles = dialog.files
      if (selectedFiles.isEmpty()) {
        currentOnResult.value(FilePickerResult.Cancelled)
      } else {
        scope.launch {
          val result =
            withContext(Dispatchers.IO) {
              aggregateSelectedFiles(
                selectedFiles.map { file ->
                  runCatching {
                    val providerMimeType = file.probeContentType()
                    val svgMimeType = svgMimeTypeOrNull(file.name, providerMimeType)
                    val imageSize =
                      when {
                        svgMimeType != null -> decodeSvgImageSize(file.readBytes())
                        contentType == "image" ->
                          file.decodeImageOrNull()?.let { it.width to it.height }
                        else -> null
                      }
                    PickedFile(
                      filename = file.name,
                      mimeType = svgMimeType ?: providerMimeType,
                      size = file.length(),
                      previewModel = file,
                      imageWidth = imageSize?.first,
                      imageHeight = imageSize?.second,
                      openSource = { file.inputStream().asSource().buffered() },
                    )
                  }
                }
              )
            }
          currentOnResult.value(result)
        }
      }
    }
  }
}

internal fun File.probeContentType(): String? {
  return runCatching { Files.probeContentType(toPath()) }.getOrNull()
}

internal fun File.decodeImageOrNull(): BufferedImage? =
  runCatching { ImageIO.read(this) }.getOrNull()
