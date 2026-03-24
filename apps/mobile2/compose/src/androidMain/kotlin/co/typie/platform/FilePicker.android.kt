package co.typie.platform

import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.platform.LocalContext

@Composable
actual fun rememberFilePicker(
  onResult: (PlatformFile?) -> Unit,
): (mimeType: String) -> Unit {
  val context = LocalContext.current
  val currentOnResult = rememberUpdatedState(onResult)
  val launcher = rememberLauncherForActivityResult(
    contract = ActivityResultContracts.GetContent(),
  ) { uri ->
    if (uri == null) {
      currentOnResult.value(null)
      return@rememberLauncherForActivityResult
    }

    val bytes = context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
    if (bytes == null) {
      currentOnResult.value(null)
      return@rememberLauncherForActivityResult
    }

    val mimeType = context.contentResolver.getType(uri)
    val filename = context.contentResolver.query(
      uri,
      arrayOf(OpenableColumns.DISPLAY_NAME),
      null,
      null,
      null,
    )?.use { cursor ->
      if (cursor.moveToFirst()) cursor.getString(0) else null
    }

    currentOnResult.value(
      PlatformFile(
        bytes = bytes,
        filename = pickedFilename(filename, mimeType),
        mimeType = mimeType,
      ),
    )
  }

  return remember(launcher) {
    { mimeType: String -> launcher.launch(mimeType) }
  }
}
