package co.typie.screen.stats

import android.content.ClipData
import android.content.ClipboardManager
import android.content.ContentValues
import android.content.Context
import android.os.Environment
import android.provider.MediaStore
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.FileProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

@Composable
actual fun rememberStatsImageExporter(): StatsImageExporter {
  val context = LocalContext.current
  return remember(context) {
    AndroidStatsImageExporter(context.applicationContext)
  }
}

private class AndroidStatsImageExporter(
  private val context: Context,
) : StatsImageExporter {

  override suspend fun copyPng(
    bytes: ByteArray,
    suggestedName: String,
  ): Boolean = withContext(Dispatchers.IO) {
    runCatching {
      val directory = File(context.cacheDir, "stats-images").apply { mkdirs() }
      val file = File(directory, ensurePngFilename(suggestedName))
      file.writeBytes(bytes)

      val uri = FileProvider.getUriForFile(
        context,
        "${context.packageName}.fileprovider",
        file,
      )

      val clipData = ClipData.newUri(context.contentResolver, suggestedName, uri)
      val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
      clipboard.setPrimaryClip(clipData)
      true
    }.getOrDefault(false)
  }

  override suspend fun savePng(
    bytes: ByteArray,
    suggestedName: String,
  ): StatsImageSaveResult = withContext(Dispatchers.IO) {
    try {
      val filename = ensurePngFilename(suggestedName)
      val resolver = context.contentResolver
      val contentValues = ContentValues().apply {
        put(MediaStore.MediaColumns.DISPLAY_NAME, filename)
        put(MediaStore.MediaColumns.MIME_TYPE, "image/png")
        put(MediaStore.MediaColumns.RELATIVE_PATH, "${Environment.DIRECTORY_PICTURES}/Typie")
        put(MediaStore.MediaColumns.IS_PENDING, 1)
      }

      val uri = resolver.insert(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, contentValues)
        ?: return@withContext StatsImageSaveResult.Error

      resolver.openOutputStream(uri)?.use { stream ->
        stream.write(bytes)
      } ?: return@withContext StatsImageSaveResult.Error

      resolver.update(
        uri,
        ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) },
        null,
        null,
      )

      StatsImageSaveResult.Success
    } catch (_: SecurityException) {
      StatsImageSaveResult.PermissionDenied
    } catch (_: Exception) {
      StatsImageSaveResult.Error
    }
  }
}

private fun ensurePngFilename(name: String): String {
  return if (name.endsWith(".png", ignoreCase = true)) name else "$name.png"
}
