package co.typie.media

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import java.awt.FileDialog
import java.awt.Frame
import java.io.File
import java.nio.file.Files

@Composable
actual fun rememberImagePicker(
  onResult: (PickedImage?) -> Unit,
): () -> Unit {
  val currentOnResult = rememberUpdatedState(onResult)

  return remember {
    {
      val dialog = FileDialog(null as Frame?, imagePickerDialogTitle, FileDialog.LOAD).apply {
        isVisible = true
      }
      val file = dialog.files.firstOrNull()

      if (file == null) {
        currentOnResult.value(null)
        return@remember
      }

      currentOnResult.value(
        PickedImage(
          bytes = file.readBytes(),
          filename = file.name,
          mimeType = file.probeContentType(),
        ),
      )
    }
  }
}

private fun File.probeContentType(): String? {
  return runCatching { Files.probeContentType(toPath()) }.getOrNull()
}
