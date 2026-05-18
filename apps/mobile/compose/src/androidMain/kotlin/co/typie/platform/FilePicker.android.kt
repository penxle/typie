package co.typie.platform

import android.content.Context
import android.graphics.BitmapFactory
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
  onResult: (List<PickedFile>) -> Unit,
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

private fun Context.readPlatformFile(uri: Uri): PickedFile? {
  val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return null
  val mimeType = contentResolver.getType(uri)
  val imageSize = bytes.decodeImageSizeIfNeeded(mimeType)
  val filename =
    contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
      cursor ->
      if (cursor.moveToFirst()) cursor.getString(0) else null
    }

  return PickedFile(
    bytes = bytes,
    filename = pickedFilename(filename, mimeType),
    mimeType = mimeType,
    imageWidth = imageSize?.first,
    imageHeight = imageSize?.second,
  )
}

private fun ByteArray.decodeImageSizeIfNeeded(mimeType: String?): Pair<Int, Int>? {
  return when (mimeType?.substringBefore('/')) {
    "image" -> {
      val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
      BitmapFactory.decodeByteArray(this, 0, size, options)
      val width = options.outWidth
      val height = options.outHeight
      if (width <= 0 || height <= 0) null else width to height
    }
    else -> null
  }
}
