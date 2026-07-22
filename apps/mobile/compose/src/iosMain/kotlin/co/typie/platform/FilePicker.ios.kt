@file:OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)

package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import kotlin.math.roundToInt
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.ObjCObjectVar
import kotlinx.cinterop.StableRef
import kotlinx.cinterop.alloc
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.useContents
import kotlinx.cinterop.value
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.cancel
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.io.buffered
import kotlinx.io.files.Path
import kotlinx.io.files.SystemFileSystem
import platform.Foundation.NSError
import platform.Foundation.NSFileManager
import platform.Foundation.NSFileSize
import platform.Foundation.NSItemProvider
import platform.Foundation.NSNumber
import platform.Foundation.NSProgress
import platform.Foundation.NSTemporaryDirectory
import platform.Foundation.NSURL
import platform.Foundation.NSUUID
import platform.PhotosUI.PHPickerConfiguration
import platform.PhotosUI.PHPickerConfigurationAssetRepresentationModeCurrent
import platform.PhotosUI.PHPickerConfigurationSelectionOrdered
import platform.PhotosUI.PHPickerFilter
import platform.PhotosUI.PHPickerResult
import platform.PhotosUI.PHPickerViewController
import platform.PhotosUI.PHPickerViewControllerDelegateProtocol
import platform.UIKit.UIApplication
import platform.UIKit.UIDocumentPickerDelegateProtocol
import platform.UIKit.UIDocumentPickerViewController
import platform.UIKit.UIImage
import platform.UIKit.UISceneActivationStateForegroundActive
import platform.UIKit.UIViewController
import platform.UIKit.UIWindow
import platform.UIKit.UIWindowScene
import platform.UniformTypeIdentifiers.UTType
import platform.UniformTypeIdentifiers.UTTypeData
import platform.UniformTypeIdentifiers.UTTypeImage
import platform.UniformTypeIdentifiers.conformsToType
import platform.darwin.NSObject
import platform.objc.OBJC_ASSOCIATION_RETAIN_NONATOMIC
import platform.objc.objc_setAssociatedObject

@Composable
actual fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode,
  onResult: (FilePickerResult) -> Unit,
): (mimeType: String) -> Unit {
  val currentOnResult = rememberUpdatedState(onResult)

  return remember(selectionMode) {
    pickerLauncher@{ mimeType: String ->
      val presenter =
        topViewController()
          ?: run {
            currentOnResult.value(
              FilePickerResult.Failed(
                IllegalStateException("No view controller can present the file picker")
              )
            )
            return@pickerLauncher
          }

      when (mimeType.substringBefore('/')) {
        "image" -> {
          val configuration =
            PHPickerConfiguration().apply {
              filter = PHPickerFilter.imagesFilter
              selectionLimit =
                when (selectionMode) {
                  FilePickerSelectionMode.Single -> 1
                  FilePickerSelectionMode.Multiple -> 0
                }
              selection = PHPickerConfigurationSelectionOrdered
              preferredAssetRepresentationMode = PHPickerConfigurationAssetRepresentationModeCurrent
            }
          val picker = PHPickerViewController(configuration)
          val session = ImagePickerSession { result -> currentOnResult.value(result) }

          picker.retainFilePickerDelegate(session)
          picker.delegate = session
          presenter.presentViewController(picker, animated = true, completion = null)
        }
        else -> {
          val types =
            listOf(
              when (mimeType) {
                "*/*" -> UTTypeData
                else -> UTType.typeWithMIMEType(mimeType) ?: UTTypeData
              }
            )
          val picker =
            UIDocumentPickerViewController(forOpeningContentTypes = types, asCopy = true).apply {
              allowsMultipleSelection = selectionMode == FilePickerSelectionMode.Multiple
            }

          val delegate = DocumentPickerDelegate { result -> currentOnResult.value(result) }
          picker.retainFilePickerDelegate(delegate)
          picker.delegate = delegate
          presenter.presentViewController(picker, animated = true, completion = null)
        }
      }
    }
  }
}

private class ImagePickerSession(private val onResult: (FilePickerResult) -> Unit) :
  NSObject(), PHPickerViewControllerDelegateProtocol {
  private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
  private var started = false
  private var finished = false

  override fun picker(picker: PHPickerViewController, didFinishPicking: List<*>) {
    if (started) return
    started = true
    picker.dismissViewControllerAnimated(true, completion = null)

    if (didFinishPicking.isEmpty()) {
      finish(picker, FilePickerResult.Cancelled)
      return
    }

    scope.launch {
      val loadedFiles = coroutineScope {
        didFinishPicking
          .map { value ->
            async {
              val result = value as? PHPickerResult
              if (result == null) {
                Result.failure(IllegalStateException("Selected image result is unavailable"))
              } else {
                try {
                  result.itemProvider.loadSelectedImage()
                } catch (error: Throwable) {
                  Result.failure(error)
                }
              }
            }
          }
          .awaitAll()
      }
      finish(picker, aggregateSelectedFiles(loadedFiles))
    }
  }

  private fun finish(picker: PHPickerViewController, result: FilePickerResult) {
    if (finished) return
    finished = true
    picker.retainFilePickerDelegate(null)
    try {
      onResult(result)
    } finally {
      scope.cancel()
    }
  }
}

private class DocumentPickerDelegate(private val onResult: (FilePickerResult) -> Unit) :
  NSObject(), UIDocumentPickerDelegateProtocol {
  override fun documentPicker(
    controller: UIDocumentPickerViewController,
    didPickDocumentsAtURLs: List<*>,
  ) {
    controller.retainFilePickerDelegate(null)
    onResult(
      aggregateSelectedFiles(
        didPickDocumentsAtURLs.map { value ->
          runCatching {
            val url = value as? NSURL ?: error("Selected document URL is unavailable")
            url.toPickedFile(owned = true)
          }
        }
      )
    )
  }

  override fun documentPickerWasCancelled(controller: UIDocumentPickerViewController) {
    controller.retainFilePickerDelegate(null)
    onResult(FilePickerResult.Cancelled)
  }
}

private suspend fun NSItemProvider.loadSelectedImage(): Result<PickedFile> {
  val typeIdentifier =
    registeredTypeIdentifiers.filterIsInstance<String>().firstOrNull { identifier ->
      UTType.typeWithIdentifier(identifier)?.conformsToType(UTTypeImage) == true
    }
      ?: return Result.failure(
        IllegalStateException("Selected item has no readable image representation")
      )
  val type = UTType.typeWithIdentifier(typeIdentifier)
  val mimeType = type?.preferredMIMEType
  val filename = providerFilename(suggestedName, type?.preferredFilenameExtension, mimeType)

  return suspendCancellableCoroutine { continuation ->
    var progress: NSProgress? = null
    continuation.invokeOnCancellation { progress?.cancel() }

    progress =
      loadFileRepresentationForTypeIdentifier(typeIdentifier) { url, providerError ->
        val result = runCatching {
          val sourceURL =
            url
              ?: error(
                providerError?.localizedDescription
                  ?: "Selected image representation is unavailable"
              )
          val ownedURL = copyToTemporaryFile(sourceURL, filename)
          try {
            ownedURL.toPickedImage(filename = filename, mimeType = mimeType)
          } catch (error: Throwable) {
            removeOwnedFile(ownedURL)
            throw error
          }
        }

        continuation.resume(result) { _, undeliveredResult, _ ->
          undeliveredResult.getOrNull()?.close()
        }
      }

    if (!continuation.isActive) {
      progress.cancel()
    }
  }
}

internal fun NSURL.toPickedImage(filename: String, mimeType: String?): PickedFile {
  val path = requireNotNull(path) { "Selected image path is unavailable" }
  val image = UIImage.imageWithContentsOfFile(path) ?: error("Unable to decode the selected image")
  return toPickedFile(
    filename = filename,
    mimeType = mimeType,
    imageWidth = image.pixelWidth(),
    imageHeight = image.pixelHeight(),
    owned = true,
  )
}

internal fun NSURL.toPickedFile(owned: Boolean): PickedFile {
  val inferredType =
    pathExtension?.takeIf(String::isNotBlank)?.let(UTType::typeWithFilenameExtension)
  return toPickedFile(
    filename = pickedFilename(lastPathComponent, inferredType?.preferredMIMEType),
    mimeType = inferredType?.preferredMIMEType,
    owned = owned,
  )
}

internal fun NSURL.toPickedFile(
  filename: String,
  mimeType: String?,
  imageWidth: Int? = null,
  imageHeight: Int? = null,
  owned: Boolean,
): PickedFile {
  val url = this
  val path = requireNotNull(path) { "Selected file path is unavailable" }
  return PickedFile(
    filename = filename,
    mimeType = mimeType,
    size = fileSize(path),
    previewModel = url,
    imageWidth = imageWidth,
    imageHeight = imageHeight,
    openSource = { SystemFileSystem.source(Path(path)).buffered() },
    release = { if (owned) removeOwnedFile(url) },
  )
}

internal fun providerFilename(
  suggestedName: String?,
  preferredExtension: String?,
  mimeType: String?,
): String {
  val original = suggestedName?.substringAfterLast('/')?.substringAfterLast('\\')
  val withExtension =
    when {
      original.isNullOrBlank() -> null
      preferredExtension.isNullOrBlank() || original.substringAfterLast('.', "").isNotBlank() ->
        original
      else -> "$original.$preferredExtension"
    }
  return pickedFilename(withExtension, mimeType)
}

internal fun copyToTemporaryFile(sourceURL: NSURL, filename: String): NSURL {
  val safeFilename = filename.replace('/', '_').replace('\\', '_')
  val destinationPath = "${NSTemporaryDirectory()}${NSUUID().UUIDString}-$safeFilename"
  val destinationURL = NSURL.fileURLWithPath(destinationPath)

  memScoped {
    val copyError = alloc<ObjCObjectVar<NSError?>>()
    copyError.value = null
    if (!NSFileManager.defaultManager.copyItemAtURL(sourceURL, destinationURL, copyError.ptr)) {
      error(copyError.value?.localizedDescription ?: "Unable to copy selected image")
    }
  }
  return destinationURL
}

private fun fileSize(path: String): Long? =
  (NSFileManager.defaultManager.attributesOfItemAtPath(path, error = null)?.get(NSFileSize)
      as? NSNumber)
    ?.longLongValue

internal fun removeOwnedFile(url: NSURL) {
  NSFileManager.defaultManager.removeItemAtURL(url, error = null)
}

private val FilePickerDelegateAssociationKey = StableRef.create(Unit).asCPointer()

private fun UIViewController.retainFilePickerDelegate(delegate: NSObject?) {
  objc_setAssociatedObject(
    `object` = this,
    key = FilePickerDelegateAssociationKey,
    value = delegate,
    policy = OBJC_ASSOCIATION_RETAIN_NONATOMIC,
  )
}

private fun topViewController(): UIViewController? {
  val scenes = UIApplication.sharedApplication.connectedScenes.filterIsInstance<UIWindowScene>()
  val activeScenes =
    scenes
      .filter { it.activationState == UISceneActivationStateForegroundActive }
      .ifEmpty { scenes }
  val windows = activeScenes.flatMap { it.windows.filterIsInstance<UIWindow>() }
  val root =
    (windows.firstOrNull { it.keyWindow } ?: windows.firstOrNull())?.rootViewController
      ?: return null

  var controller = root
  while (controller.presentedViewController != null) {
    controller = controller.presentedViewController!!
  }
  return controller
}

private fun UIImage.pixelWidth(): Int = size.useContents { (width * scale).roundToInt() }

private fun UIImage.pixelHeight(): Int = size.useContents { (height * scale).roundToInt() }
