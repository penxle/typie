@file:OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)

package co.typie.platform

import kotlin.coroutines.resume
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import platform.Foundation.NSBundle
import platform.Foundation.NSData
import platform.Foundation.NSDocumentDirectory
import platform.Foundation.NSSearchPathForDirectoriesInDomains
import platform.Foundation.NSUserDomainMask
import platform.Foundation.create
import platform.Foundation.writeToFile
import platform.Photos.PHAuthorizationStatusAuthorized
import platform.Photos.PHAuthorizationStatusDenied
import platform.Photos.PHAuthorizationStatusLimited
import platform.Photos.PHAuthorizationStatusNotDetermined
import platform.Photos.PHAuthorizationStatusRestricted
import platform.Photos.PHPhotoLibrary
import platform.UIKit.UIActivityViewController
import platform.UIKit.UIApplication
import platform.UIKit.UIDevice
import platform.UIKit.UIImage
import platform.UIKit.UIImageWriteToSavedPhotosAlbum
import platform.UIKit.UIPasteboard
import platform.UIKit.UIViewController

private fun NSBundle.infoString(key: String): String =
  (objectForInfoDictionaryKey(key) as? String)?.takeIf(String::isNotBlank) ?: "unknown"

internal class IOSDeviceInfo : DeviceInfo {
  override fun retrieve(): DeviceInfoData {
    val device = UIDevice.currentDevice
    val bundle = NSBundle.mainBundle
    val versionName = bundle.infoString("CFBundleShortVersionString")
    val buildNumber = bundle.infoString("CFBundleVersion")

    return DeviceInfoData(
      model = device.model,
      osName = device.systemName,
      osVersion = device.systemVersion,
      appVersion = versionName,
      appBuildNumber = buildNumber,
    )
  }
}

internal class IOSClipboard : Clipboard {
  override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean =
    withContext(Dispatchers.Default) {
      runCatching {
          if (mimeType.startsWith("image/")) {
            UIPasteboard.generalPasteboard.image = bytes.toUIImage()
          } else {
            UIPasteboard.generalPasteboard.setData(bytes.toNSData(), forPasteboardType = mimeType)
          }
          true
        }
        .getOrDefault(false)
    }

  override suspend fun copy(text: String, mimeType: String): Boolean =
    withContext(Dispatchers.Default) {
      runCatching {
          UIPasteboard.generalPasteboard.string = text
          true
        }
        .getOrDefault(false)
    }

  override suspend fun copyRichText(html: String, text: String): Boolean =
    withContext(Dispatchers.Default) {
      runCatching {
          UIPasteboard.generalPasteboard.setItems(
            listOf(mapOf(UTI_HTML to html, UTI_PLAIN_TEXT to text))
          )
          true
        }
        .getOrDefault(false)
    }

  override suspend fun paste(): ClipboardReadPayload? =
    withContext(Dispatchers.Default) {
      runCatching {
          val pasteboard = UIPasteboard.generalPasteboard
          val html = pasteboard.valueForPasteboardType(UTI_HTML) as? String
          val text =
            (pasteboard.valueForPasteboardType(UTI_PLAIN_TEXT) as? String)
              ?: pasteboard.string
              ?: return@runCatching null
          ClipboardReadPayload(html = html, text = text)
        }
        .getOrNull()
    }
}

private const val UTI_HTML = "public.html"
private const val UTI_PLAIN_TEXT = "public.utf8-plain-text"

internal class IOSFileSystem : FileSystem {
  override suspend fun save(
    bytes: ByteArray,
    name: String,
    location: FileSystemSaveLocation,
  ): FileSystemSaveResult =
    withContext(Dispatchers.Default) {
      try {
        when (location) {
          FileSystemSaveLocation.Gallery -> {
            val image = bytes.toUIImage()
            val granted = requestPhotoLibraryAccess()
            if (!granted) return@withContext FileSystemSaveResult.PermissionDenied
            UIImageWriteToSavedPhotosAlbum(image, null, null, null)
            FileSystemSaveResult.Success
          }

          FileSystemSaveLocation.Files -> {
            val paths =
              NSSearchPathForDirectoriesInDomains(NSDocumentDirectory, NSUserDomainMask, true)
            val documentsDir =
              paths.firstOrNull() as? String ?: return@withContext FileSystemSaveResult.Error
            val filePath = "$documentsDir/$name"
            val data = bytes.toNSData()
            val success = data.writeToFile(filePath, atomically = true)
            if (success) FileSystemSaveResult.Success else FileSystemSaveResult.Error
          }
        }
      } catch (_: Exception) {
        FileSystemSaveResult.Error
      }
    }
}

private suspend fun requestPhotoLibraryAccess(): Boolean =
  suspendCancellableCoroutine { continuation ->
    when (PHPhotoLibrary.authorizationStatus()) {
      PHAuthorizationStatusAuthorized,
      PHAuthorizationStatusLimited -> continuation.resume(true)
      PHAuthorizationStatusDenied,
      PHAuthorizationStatusRestricted -> continuation.resume(false)
      PHAuthorizationStatusNotDetermined -> {
        PHPhotoLibrary.requestAuthorization { status ->
          continuation.resume(
            status == PHAuthorizationStatusAuthorized || status == PHAuthorizationStatusLimited
          )
        }
      }

      else -> continuation.resume(false)
    }
  }

private fun ByteArray.toUIImage(): UIImage {
  return UIImage(data = toNSData())
}

private fun ByteArray.toNSData(): NSData {
  return usePinned { pinned -> NSData.create(bytes = pinned.addressOf(0), length = size.toULong()) }
}

internal class IOSShare : Share {
  override suspend fun share(bytes: ByteArray, mimeType: String): Boolean =
    withContext(Dispatchers.Main) {
      runCatching {
          val item: Any =
            if (mimeType.startsWith("image/")) {
              bytes.toUIImage()
            } else {
              bytes.toNSData()
            }

          presentShareSheet(listOf(item))
          true
        }
        .getOrDefault(false)
    }

  override suspend fun share(text: String): Boolean =
    withContext(Dispatchers.Main) {
      runCatching {
          presentShareSheet(listOf(text))
          true
        }
        .getOrDefault(false)
    }

  private fun presentShareSheet(items: List<Any>) {
    val controller = topViewController() ?: return
    val activityVC = UIActivityViewController(activityItems = items, applicationActivities = null)
    controller.presentViewController(activityVC, animated = true, completion = null)
  }

  private fun topViewController(): UIViewController? {
    var controller = UIApplication.sharedApplication.keyWindow?.rootViewController ?: return null
    while (controller.presentedViewController != null) {
      controller = controller.presentedViewController!!
    }
    return controller
  }
}
