package co.typie.platform

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.platform.LocalContext

@Composable
actual fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode,
  onResult: (List<PlatformFile>) -> Unit,
): (mimeType: String) -> Unit {
  val context = LocalContext.current
  val currentOnResult = rememberUpdatedState(onResult)
  val singleLauncher =
    rememberLauncherForActivityResult(contract = ActivityResultContracts.GetContent()) { uri ->
      currentOnResult.value(uri?.let { context.readPlatformFile(it) }?.let(::listOf) ?: emptyList())
    }
  val multipleLauncher =
    rememberLauncherForActivityResult(contract = ActivityResultContracts.GetMultipleContents()) {
      uris ->
      currentOnResult.value(uris.mapNotNull(context::readPlatformFile))
    }

  return remember(singleLauncher, multipleLauncher, selectionMode) {
    { mimeType: String ->
      when (selectionMode) {
        FilePickerSelectionMode.Single -> singleLauncher.launch(mimeType)
        FilePickerSelectionMode.Multiple -> multipleLauncher.launch(mimeType)
      }
    }
  }
}

private fun Context.readPlatformFile(uri: Uri): PlatformFile? {
  val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return null
  val mimeType = contentResolver.getType(uri)
  val filename =
    contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
      cursor ->
      if (cursor.moveToFirst()) cursor.getString(0) else null
    }

  return PlatformFile(
    bytes = bytes,
    filename = pickedFilename(filename, mimeType),
    mimeType = mimeType,
  )
}
