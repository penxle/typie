package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import java.awt.FileDialog
import java.awt.Frame
import java.io.File
import java.nio.file.Files

@Composable
actual fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode,
  onResult: (List<PlatformFile>) -> Unit,
): (mimeType: String) -> Unit {
  val currentOnResult = rememberUpdatedState(onResult)

  return remember(selectionMode) {
    { mimeType: String ->
      val title = if (mimeType.startsWith("image/")) "이미지 선택" else "파일 선택"
      val dialog = FileDialog(null as Frame?, title, FileDialog.LOAD).apply {
        if (mimeType.startsWith("image/")) {
          setFilenameFilter { _, name ->
            val lower = name.lowercase()
            lower.endsWith(".png") || lower.endsWith(".jpg") || lower.endsWith(".jpeg") ||
              lower.endsWith(".webp") || lower.endsWith(".heic")
          }
        }
        isMultipleMode = selectionMode == FilePickerSelectionMode.Multiple
        isVisible = true
      }
      val files = dialog.files
        .map { file ->
          PlatformFile(
            bytes = file.readBytes(),
            filename = file.name,
            mimeType = file.probeContentType(),
          )
        }

      currentOnResult.value(files)
    }
  }
}

private fun File.probeContentType(): String? {
  return runCatching { Files.probeContentType(toPath()) }.getOrNull()
}
