package co.typie.platform

import android.content.Context
import android.graphics.BitmapFactory
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.platform.LocalContext
import java.io.File
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
  val context = LocalContext.current
  val scope = rememberCoroutineScope()
  val currentOnResult = rememberUpdatedState(onResult)
  val singleLauncher =
    rememberLauncherForActivityResult(contract = ActivityResultContracts.GetContent()) { uri ->
      if (uri == null) {
        currentOnResult.value(FilePickerResult.Cancelled)
      } else {
        scope.launch { currentOnResult.value(context.readSelectedFiles(listOf(uri))) }
      }
    }
  val multipleLauncher =
    rememberLauncherForActivityResult(contract = ActivityResultContracts.GetMultipleContents()) {
      uris ->
      if (uris.isEmpty()) {
        currentOnResult.value(FilePickerResult.Cancelled)
      } else {
        scope.launch { currentOnResult.value(context.readSelectedFiles(uris)) }
      }
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

private suspend fun Context.readSelectedFiles(uris: List<Uri>): FilePickerResult =
  withContext(Dispatchers.IO) {
    aggregateSelectedFiles(uris.map { uri -> runCatching { readPlatformFile(uri) } })
  }

internal fun Context.readPlatformFile(
  uri: Uri,
  fallbackMimeType: String? = null,
  release: () -> Unit = {},
): PickedFile {
  val metadata = queryMetadata(uri)
  val providerMimeType = contentResolver.getType(uri) ?: fallbackMimeType
  val svgMimeType = svgMimeTypeOrNull(metadata.filename, providerMimeType)
  val mimeType = svgMimeType ?: providerMimeType
  val imageSize =
    if (svgMimeType != null) {
      val bytes =
        contentResolver.openInputStream(uri)?.use { it.readBytes() }
          ?: error("Unable to open selected image")
      decodeSvgImageSize(bytes)
    } else {
      decodeImageSizeIfNeeded(uri, mimeType)
    }

  if (mimeType?.substringBefore('/') != "image") {
    contentResolver.openInputStream(uri)?.close() ?: error("Unable to open selected file")
  }

  return PickedFile(
    filename = pickedFilename(metadata.filename, mimeType),
    mimeType = mimeType,
    size = metadata.size,
    previewModel = uri,
    imageWidth = imageSize?.first,
    imageHeight = imageSize?.second,
    openSource = {
      contentResolver.openInputStream(uri)?.asSource()?.buffered()
        ?: error("Unable to open selected file")
    },
    release = release,
  )
}

internal fun Context.copyIncomingContentItem(
  uri: Uri,
  fallbackMimeType: String? = null,
): IncomingContentItem {
  val metadata = queryMetadata(uri)
  val providerMimeType = contentResolver.getType(uri) ?: fallbackMimeType
  val svgMimeType = svgMimeTypeOrNull(metadata.filename, providerMimeType)
  val mimeType = svgMimeType ?: providerMimeType
  val directory = File(cacheDir, "incoming-content").apply { mkdirs() }
  val ownedFile = File.createTempFile("incoming-", ".tmp", directory)

  try {
    contentResolver.openInputStream(uri)?.use { input ->
      ownedFile.outputStream().use { output -> input.copyTo(output) }
    } ?: error("Unable to open incoming file")
    val imageSize =
      when {
        svgMimeType != null -> decodeSvgImageSize(ownedFile.readBytes())
        mimeType?.substringBefore('/') == "image" -> {
          val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
          BitmapFactory.decodeFile(ownedFile.path, options)
          options.outWidth
            .takeIf { it > 0 }
            ?.let { width ->
              options.outHeight.takeIf { it > 0 }?.let { height -> width to height }
            }
        }
        else -> null
      }

    return IncomingContentItem(
      kind =
        if (mimeType?.substringBefore('/') == "image") {
          IncomingContentItem.Kind.Image
        } else {
          IncomingContentItem.Kind.File
        },
      file =
        PickedFile(
          filename = pickedFilename(metadata.filename, mimeType),
          mimeType = mimeType,
          size = metadata.size ?: ownedFile.length(),
          previewModel = ownedFile,
          imageWidth = imageSize?.first,
          imageHeight = imageSize?.second,
          openSource = { ownedFile.inputStream().asSource().buffered() },
          release = { ownedFile.delete() },
        ),
    )
  } catch (error: Throwable) {
    ownedFile.delete()
    throw error
  }
}

private data class FileMetadata(val filename: String?, val size: Long?)

private fun Context.queryMetadata(uri: Uri): FileMetadata {
  return contentResolver
    .query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)
    ?.use { cursor ->
      if (!cursor.moveToFirst()) return@use FileMetadata(filename = null, size = null)
      val filenameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
      val filename =
        if (filenameIndex >= 0 && !cursor.isNull(filenameIndex)) cursor.getString(filenameIndex)
        else null
      val sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE)
      val size =
        if (sizeIndex >= 0 && !cursor.isNull(sizeIndex)) cursor.getLong(sizeIndex) else null
      FileMetadata(filename = filename, size = size)
    } ?: FileMetadata(filename = null, size = null)
}

private fun Context.decodeImageSizeIfNeeded(uri: Uri, mimeType: String?): Pair<Int, Int>? {
  return when (mimeType?.substringBefore('/')) {
    "image" -> {
      val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
      val stream = contentResolver.openInputStream(uri) ?: error("Unable to open selected image")
      // decodeStream always returns null when inJustDecodeBounds is set; success is indicated by
      // outWidth/outHeight instead.
      stream.use { BitmapFactory.decodeStream(it, null, options) }
      val width = options.outWidth
      val height = options.outHeight
      if (width <= 0 || height <= 0) null else width to height
    }
    else -> null
  }
}
