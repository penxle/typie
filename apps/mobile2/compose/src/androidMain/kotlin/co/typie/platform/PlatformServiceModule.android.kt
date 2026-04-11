package co.typie.platform

import android.content.ClipData
import android.content.ClipboardManager
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import androidx.core.content.FileProvider
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

internal class AndroidDeviceInfo(private val context: Context) : DeviceInfo {
  override fun retrieve(): DeviceInfoData {
    val packageInfo = context.packageManager.getPackageInfo(context.packageName, 0)
    val versionCode = packageInfo.longVersionCode.toString()
    val versionName = packageInfo.versionName?.takeIf { it.isNotBlank() } ?: "unknown"

    val manufacturer = Build.MANUFACTURER?.trim().orEmpty()
    val model = Build.MODEL?.trim().orEmpty()
    val modelName = "$manufacturer $model".trim().ifBlank { Build.DEVICE ?: "Android device" }

    return DeviceInfoData(
      model = modelName,
      osName = "Android",
      osVersion = Build.VERSION.RELEASE ?: "unknown",
      appVersion = versionName,
      appBuildNumber = versionCode,
    )
  }
}

internal class AndroidClipboard(private val context: Context) : Clipboard {
  override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean =
    withContext(Dispatchers.IO) {
      runCatching {
          val directory = File(context.cacheDir, "clipboard").apply { mkdirs() }
          val extension = mimeType.substringAfter('/').substringBefore(';')
          val file = File(directory, "clipboard.$extension")
          file.writeBytes(bytes)

          val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)

          val clipData = ClipData.newUri(context.contentResolver, "clipboard", uri)
          val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
          clipboard.setPrimaryClip(clipData)
          true
        }
        .getOrDefault(false)
    }

  override suspend fun copy(text: String, mimeType: String): Boolean =
    withContext(Dispatchers.IO) {
      runCatching {
          val clipData = ClipData.newPlainText("clipboard", text)
          val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
          clipboard.setPrimaryClip(clipData)
          true
        }
        .getOrDefault(false)
    }
}

internal class AndroidFileSystem(private val context: Context) : FileSystem {
  override suspend fun save(
    bytes: ByteArray,
    name: String,
    location: FileSystemSaveLocation,
  ): FileSystemSaveResult =
    withContext(Dispatchers.IO) {
      try {
        val (collection, relativePath) =
          when (location) {
            FileSystemSaveLocation.Gallery -> {
              MediaStore.Images.Media.EXTERNAL_CONTENT_URI to
                "${Environment.DIRECTORY_PICTURES}/Typie"
            }
            FileSystemSaveLocation.Files -> {
              MediaStore.Downloads.EXTERNAL_CONTENT_URI to Environment.DIRECTORY_DOWNLOADS
            }
          }

        val mimeType =
          when {
            name.endsWith(".png", ignoreCase = true) -> "image/png"
            name.endsWith(".jpg", ignoreCase = true) || name.endsWith(".jpeg", ignoreCase = true) ->
              "image/jpeg"
            name.endsWith(".webp", ignoreCase = true) -> "image/webp"
            else -> "application/octet-stream"
          }

        val resolver = context.contentResolver
        val contentValues =
          ContentValues().apply {
            put(MediaStore.MediaColumns.DISPLAY_NAME, name)
            put(MediaStore.MediaColumns.MIME_TYPE, mimeType)
            put(MediaStore.MediaColumns.RELATIVE_PATH, relativePath)
            put(MediaStore.MediaColumns.IS_PENDING, 1)
          }

        val uri =
          resolver.insert(collection, contentValues)
            ?: return@withContext FileSystemSaveResult.Error

        resolver.openOutputStream(uri)?.use { stream -> stream.write(bytes) }
          ?: return@withContext FileSystemSaveResult.Error

        resolver.update(
          uri,
          ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) },
          null,
          null,
        )

        FileSystemSaveResult.Success
      } catch (_: SecurityException) {
        FileSystemSaveResult.PermissionDenied
      } catch (_: Exception) {
        FileSystemSaveResult.Error
      }
    }
}

internal class AndroidShare(private val context: Context) : Share {
  override suspend fun share(bytes: ByteArray, mimeType: String): Boolean =
    withContext(Dispatchers.IO) {
      runCatching {
          val directory = File(context.cacheDir, "share").apply { mkdirs() }
          val extension = mimeType.substringAfter('/').substringBefore(';')
          val file = File(directory, "share.$extension")
          file.writeBytes(bytes)

          val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)

          val intent =
            Intent(Intent.ACTION_SEND).apply {
              type = mimeType
              putExtra(Intent.EXTRA_STREAM, uri)
              addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
              addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }

          context.startActivity(
            Intent.createChooser(intent, null).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
          )
          true
        }
        .getOrDefault(false)
    }

  override suspend fun share(text: String): Boolean =
    withContext(Dispatchers.IO) {
      runCatching {
          val intent =
            Intent(Intent.ACTION_SEND).apply {
              type = "text/plain"
              putExtra(Intent.EXTRA_TEXT, text)
              addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }

          context.startActivity(
            Intent.createChooser(intent, null).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
          )
          true
        }
        .getOrDefault(false)
    }
}
