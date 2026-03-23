@file:OptIn(ExperimentalForeignApi::class, kotlinx.cinterop.BetaInteropApi::class)

package co.typie.screen.stats

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import platform.Foundation.NSData
import platform.Foundation.create
import platform.Photos.PHAuthorizationStatusAuthorized
import platform.Photos.PHAuthorizationStatusDenied
import platform.Photos.PHAuthorizationStatusLimited
import platform.Photos.PHAuthorizationStatusNotDetermined
import platform.Photos.PHAuthorizationStatusRestricted
import platform.Photos.PHPhotoLibrary
import platform.UIKit.UIImage
import platform.UIKit.UIImageWriteToSavedPhotosAlbum
import platform.UIKit.UIPasteboard
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

@Composable
actual fun rememberStatsImageExporter(): StatsImageExporter {
  return remember { IOSStatsImageExporter() }
}

private class IOSStatsImageExporter : StatsImageExporter {
  override suspend fun copyPng(
    bytes: ByteArray,
    suggestedName: String,
  ): Boolean = withContext(Dispatchers.Default) {
    val image = bytes.toUIImage() ?: return@withContext false
    UIPasteboard.generalPasteboard.image = image
    true
  }

  override suspend fun savePng(
    bytes: ByteArray,
    suggestedName: String,
  ): StatsImageSaveResult {
    val image = bytes.toUIImage() ?: return StatsImageSaveResult.Error

    return requestPhotoLibraryAccess().let { granted ->
      if (!granted) {
        StatsImageSaveResult.PermissionDenied
      } else {
        UIImageWriteToSavedPhotosAlbum(image, null, null, null)
        StatsImageSaveResult.Success
      }
    }
  }
}

private suspend fun requestPhotoLibraryAccess(): Boolean = suspendCancellableCoroutine { continuation ->
  when (PHPhotoLibrary.authorizationStatus()) {
    PHAuthorizationStatusAuthorized, PHAuthorizationStatusLimited -> continuation.resume(true)
    PHAuthorizationStatusDenied, PHAuthorizationStatusRestricted -> continuation.resume(false)
    PHAuthorizationStatusNotDetermined -> {
      PHPhotoLibrary.requestAuthorization { status ->
        continuation.resume(status == PHAuthorizationStatusAuthorized || status == PHAuthorizationStatusLimited)
      }
    }

    else -> continuation.resume(false)
  }
}

private fun ByteArray.toUIImage(): UIImage? {
  return UIImage(data = toNSData())
}

private fun ByteArray.toNSData(): NSData {
  return usePinned { pinned ->
    NSData.create(
      bytes = pinned.addressOf(0),
      length = size.toULong(),
    )
  }
}
