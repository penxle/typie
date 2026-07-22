@file:OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)

package co.typie.platform

import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.readBytes
import kotlinx.cinterop.useContents
import kotlinx.cinterop.usePinned
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import platform.CoreGraphics.CGRectMake
import platform.Foundation.NSBundle
import platform.Foundation.NSData
import platform.Foundation.NSDocumentDirectory
import platform.Foundation.NSItemProvider
import platform.Foundation.NSProgress
import platform.Foundation.NSSearchPathForDirectoriesInDomains
import platform.Foundation.NSTemporaryDirectory
import platform.Foundation.NSURL
import platform.Foundation.NSUUID
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
import platform.UIKit.UIImagePNGRepresentation
import platform.UIKit.UIImageWriteToSavedPhotosAlbum
import platform.UIKit.UIPasteboard
import platform.UIKit.UIViewController
import platform.UIKit.popoverPresentationController
import platform.UniformTypeIdentifiers.UTType
import platform.UniformTypeIdentifiers.UTTypeData
import platform.UniformTypeIdentifiers.UTTypeImage
import platform.UniformTypeIdentifiers.UTTypeText
import platform.UniformTypeIdentifiers.conformsToType

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

  override suspend fun paste(): IncomingContentCandidates? {
    val snapshot =
      withContext(Dispatchers.Main) {
        runCatching {
            val pasteboard = UIPasteboard.generalPasteboard
            val html =
              pasteboard
                .valueForPasteboardType(UTI_HTML)
                .asPasteboardString()
                ?.takeIf(String::isNotEmpty)
                ?: pasteboard
                  .dataForPasteboardType(UTI_HTML)
                  ?.decodeUtf8()
                  ?.takeIf(String::isNotEmpty)
            if (html != null) {
              return@runCatching IOSPasteboardSnapshot(
                html = html,
                text = pasteboard.string,
                items = emptyList(),
                providers = emptyList(),
              )
            }

            val pasteboardItems =
              pasteboard.items.mapNotNull { it as? Map<*, *> }.map { it.toMap() }
            IOSPasteboardSnapshot(
              html = null,
              text =
                pasteboardItems.firstNotNullOfOrNull { item ->
                  item[UTI_PLAIN_TEXT].asPasteboardString()
                } ?: pasteboard.string,
              items = pasteboardItems,
              providers =
                pasteboard.itemProviders
                  .filterIsInstance<NSItemProvider>()
                  .takeIf { it.size == pasteboardItems.size }
                  .orEmpty(),
            )
          }
          .getOrNull()
      } ?: return null

    return try {
      materializeIncomingContentCandidates(html = snapshot.html, text = snapshot.text) {
        loadOwnedIncomingContentItems(
          loaders =
            snapshot.items.mapIndexed { index, item ->
              suspend {
                val provider = snapshot.providers.getOrNull(index)
                if (provider == null) {
                  readPasteboardAttachment(item)
                } else {
                  try {
                    provider.loadPasteboardAttachment() ?: readPasteboardAttachment(item)
                  } catch (error: CancellationException) {
                    throw error
                  } catch (error: Throwable) {
                    runCatching { readPasteboardAttachment(item) }.getOrNull() ?: throw error
                  }
                }
              }
            },
          loaderContext = Dispatchers.Default,
        )
      }
    } catch (error: CancellationException) {
      throw error
    } catch (_: Throwable) {
      null
    }
  }
}

private const val UTI_HTML = "public.html"
private const val UTI_PLAIN_TEXT = "public.utf8-plain-text"
private const val UTI_FILE_URL = "public.file-url"
private const val UTI_URL = "public.url"

private data class IOSPasteboardSnapshot(
  val html: String?,
  val text: String?,
  val items: List<Map<*, *>>,
  val providers: List<NSItemProvider>,
)

private suspend fun NSItemProvider.loadPasteboardAttachment(): IncomingContentItem? {
  val representation =
    registeredTypeIdentifiers
      .filterIsInstance<String>()
      .mapNotNull { identifier ->
        val type = UTType.typeWithIdentifier(identifier) ?: return@mapNotNull null
        when {
          type.conformsToType(UTTypeImage) ->
            PasteboardProviderRepresentation(
              identifier = identifier,
              type = type,
              kind = IncomingContentItem.Kind.Image,
            )
          identifier == UTI_FILE_URL || identifier == UTI_URL || type.conformsToType(UTTypeText) ->
            null
          type.conformsToType(UTTypeData) ->
            PasteboardProviderRepresentation(
              identifier = identifier,
              type = type,
              kind = IncomingContentItem.Kind.File,
            )
          else -> null
        }
      }
      .minByOrNull { if (it.kind == IncomingContentItem.Kind.Image) 0 else 1 } ?: return null
  val mimeType = representation.type.preferredMIMEType
  val filename =
    providerFilename(
      suggestedName = suggestedName,
      preferredExtension = representation.type.preferredFilenameExtension,
      mimeType = mimeType,
    )

  return suspendCancellableCoroutine { continuation ->
    var progress: NSProgress? = null
    continuation.invokeOnCancellation { progress?.cancel() }
    progress =
      loadFileRepresentationForTypeIdentifier(representation.identifier) { url, providerError ->
        runCatching {
            val sourceURL =
              url
                ?: error(
                  providerError?.localizedDescription
                    ?: "Clipboard file representation is unavailable"
                )
            val ownedURL = copyToTemporaryFile(sourceURL, filename)
            try {
              IncomingContentItem(
                kind = representation.kind,
                file =
                  when (representation.kind) {
                    IncomingContentItem.Kind.Image -> ownedURL.toPickedImage(filename, mimeType)
                    IncomingContentItem.Kind.File ->
                      ownedURL.toPickedFile(filename = filename, mimeType = mimeType, owned = true)
                  },
              )
            } catch (error: Throwable) {
              removeOwnedFile(ownedURL)
              throw error
            }
          }
          .onSuccess { item ->
            continuation.resume(item) { _, undeliveredItem, _ -> undeliveredItem.file.close() }
          }
          .onFailure(continuation::resumeWithException)
      }
    if (!continuation.isActive) progress.cancel()
  }
}

private data class PasteboardProviderRepresentation(
  val identifier: String,
  val type: UTType,
  val kind: IncomingContentItem.Kind,
)

private fun readPasteboardAttachment(item: Map<*, *>): IncomingContentItem? {
  val fileURL = item[UTI_FILE_URL].asPasteboardFileURL()?.takeIf { it.scheme == "file" }
  if (fileURL != null) {
    val inferredType =
      fileURL.pathExtension?.takeIf(String::isNotBlank)?.let(UTType::typeWithFilenameExtension)
    val filename = pickedFilename(fileURL.lastPathComponent, inferredType?.preferredMIMEType)
    val ownedURL = copyToTemporaryFile(fileURL, filename)
    return try {
      val isImage = inferredType?.conformsToType(UTTypeImage) == true
      IncomingContentItem(
        kind = if (isImage) IncomingContentItem.Kind.Image else IncomingContentItem.Kind.File,
        file =
          if (isImage) {
            ownedURL.toPickedImage(filename, inferredType.preferredMIMEType)
          } else {
            ownedURL.toPickedFile(
              filename = filename,
              mimeType = inferredType?.preferredMIMEType,
              owned = true,
            )
          },
      )
    } catch (error: Throwable) {
      removeOwnedFile(ownedURL)
      throw error
    }
  }

  val imageValue =
    item.entries.firstNotNullOfOrNull { (rawType, value) ->
      val type = rawType as? String ?: return@firstNotNullOfOrNull null
      val uniformType = UTType.typeWithIdentifier(type) ?: return@firstNotNullOfOrNull null
      value.takeIf { uniformType.conformsToType(UTTypeImage) }
    } ?: return null
  val image =
    when (imageValue) {
      is UIImage -> imageValue
      is NSData -> UIImage(data = imageValue)
      else -> null
    } ?: error("Clipboard image representation is unreadable")
  return IncomingContentItem(
    kind = IncomingContentItem.Kind.Image,
    file = image.toClipboardPickedFile(),
  )
}

private fun Any?.asPasteboardString(): String? =
  when (this) {
    is String -> this
    is NSData -> decodeUtf8()
    else -> null
  }

private fun Any?.asPasteboardFileURL(): NSURL? =
  when (this) {
    is NSURL -> this
    is String -> NSURL(string = this)
    is NSData -> decodeUtf8()?.let { NSURL(string = it) }
    else -> null
  }

private fun UIImage.toClipboardPickedFile(): PickedFile {
  val data = UIImagePNGRepresentation(this) ?: error("Unable to encode clipboard image")
  val path = "${NSTemporaryDirectory()}${NSUUID().UUIDString}-image.png"
  check(data.writeToFile(path, atomically = true)) { "Unable to copy clipboard image" }
  val url = NSURL.fileURLWithPath(path)
  return try {
    url.toPickedImage(filename = "image.png", mimeType = "image/png")
  } catch (error: Throwable) {
    removeOwnedFile(url)
    throw error
  }
}

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

private fun NSData.decodeUtf8(): String? = bytes?.readBytes(length.toInt())?.decodeToString()

internal class IOSShare : Share {
  override suspend fun share(bytes: ByteArray, mimeType: String, anchor: ShareAnchor?): Boolean =
    withContext(Dispatchers.Main) {
      runCatching {
          val item: Any =
            if (mimeType.startsWith("image/")) {
              bytes.toUIImage()
            } else {
              bytes.toNSData()
            }

          presentShareSheet(listOf(item), anchor)
          true
        }
        .getOrDefault(false)
    }

  override suspend fun share(text: String, anchor: ShareAnchor?): Boolean =
    withContext(Dispatchers.Main) {
      runCatching {
          presentShareSheet(listOf(text), anchor)
          true
        }
        .getOrDefault(false)
    }

  private fun presentShareSheet(items: List<Any>, anchor: ShareAnchor?) {
    val controller = topViewController() ?: return
    val sourceView = controller.view
    val activityVC = UIActivityViewController(activityItems = items, applicationActivities = null)
    activityVC.popoverPresentationController?.let { popover ->
      popover.sourceView = sourceView
      if (anchor != null) {
        popover.sourceRect =
          sourceView.convertRect(
            CGRectMake(anchor.x, anchor.y, anchor.width, anchor.height),
            fromView = null,
          )
      } else {
        popover.sourceRect =
          sourceView.bounds.useContents {
            CGRectMake(size.width / 2.0, size.height / 2.0, 0.0, 0.0)
          }
        popover.permittedArrowDirections = 0uL
      }
    }
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
