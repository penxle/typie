@file:OptIn(ExperimentalForeignApi::class, BetaInteropApi::class)

package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import kotlin.math.roundToInt
import kotlinx.cinterop.BetaInteropApi
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.StableRef
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.useContents
import kotlinx.cinterop.usePinned
import platform.Foundation.NSData
import platform.Foundation.NSURL
import platform.Foundation.create
import platform.UIKit.UIApplication
import platform.UIKit.UIDocumentPickerDelegateProtocol
import platform.UIKit.UIDocumentPickerViewController
import platform.UIKit.UIImage
import platform.UIKit.UIImageJPEGRepresentation
import platform.UIKit.UIImagePickerController
import platform.UIKit.UIImagePickerControllerDelegateProtocol
import platform.UIKit.UIImagePickerControllerOriginalImage
import platform.UIKit.UIImagePickerControllerSourceType
import platform.UIKit.UINavigationControllerDelegateProtocol
import platform.UIKit.UIViewController
import platform.UniformTypeIdentifiers.UTType
import platform.UniformTypeIdentifiers.UTTypeData
import platform.darwin.NSObject
import platform.objc.OBJC_ASSOCIATION_RETAIN_NONATOMIC
import platform.objc.objc_setAssociatedObject
import platform.posix.memcpy

@Composable
actual fun rememberFilePicker(
  selectionMode: FilePickerSelectionMode,
  onResult: (FilePickerResult) -> Unit,
): (mimeType: String) -> Unit {
  val currentOnResult = rememberUpdatedState(onResult)

  return remember(selectionMode) {
    { mimeType: String ->
      val presenter =
        topViewController()
          ?: run {
            currentOnResult.value(
              FilePickerResult.Failed(
                IllegalStateException("No view controller can present the file picker")
              )
            )
            return@remember
          }

      when (mimeType.substringBefore('/')) {
        "image" -> {
          val picker =
            UIImagePickerController().apply {
              sourceType =
                UIImagePickerControllerSourceType.UIImagePickerControllerSourceTypePhotoLibrary
            }

          val delegate =
            object :
              NSObject(),
              UIImagePickerControllerDelegateProtocol,
              UINavigationControllerDelegateProtocol {
              override fun imagePickerControllerDidCancel(picker: UIImagePickerController) {
                picker.dismissViewControllerAnimated(true, completion = null)
                picker.retainFilePickerDelegate(null)
                currentOnResult.value(FilePickerResult.Cancelled)
              }

              override fun imagePickerController(
                picker: UIImagePickerController,
                didFinishPickingMediaWithInfo: Map<Any?, *>,
              ) {
                val image =
                  didFinishPickingMediaWithInfo[UIImagePickerControllerOriginalImage] as? UIImage
                val data = image?.let { UIImageJPEGRepresentation(it, 0.9) }

                picker.dismissViewControllerAnimated(true, completion = null)
                picker.retainFilePickerDelegate(null)

                if (data == null) {
                  currentOnResult.value(
                    FilePickerResult.Failed(
                      IllegalStateException("Unable to convert the selected image")
                    )
                  )
                  return
                }

                currentOnResult.value(
                  FilePickerResult.Selected(
                    files =
                      listOf(
                        PickedFile(
                          bytes = data.toByteArray(),
                          filename =
                            pickedFilename(originalFilename = null, mimeType = "image/jpeg"),
                          mimeType = "image/jpeg",
                          imageWidth = image.pixelWidth(),
                          imageHeight = image.pixelHeight(),
                        )
                      )
                  )
                )
              }
            }

          picker.retainFilePickerDelegate(delegate)
          picker.delegate = delegate
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

          val delegate =
            object : NSObject(), UIDocumentPickerDelegateProtocol {
              override fun documentPicker(
                controller: UIDocumentPickerViewController,
                didPickDocumentsAtURLs: List<*>,
              ) {
                controller.retainFilePickerDelegate(null)
                currentOnResult.value(
                  aggregateSelectedFiles(
                    didPickDocumentsAtURLs.map { value ->
                      runCatching {
                        val url = value as? NSURL ?: error("Selected document URL is unavailable")
                        val data =
                          NSData.create(contentsOfURL = url)
                            ?: error("Unable to read selected document")
                        PickedFile(
                          bytes = data.toByteArray(),
                          filename = pickedFilename(url.lastPathComponent, mimeType = null),
                          mimeType = mimeType,
                        )
                      }
                    }
                  )
                )
              }

              override fun documentPickerWasCancelled(controller: UIDocumentPickerViewController) {
                controller.retainFilePickerDelegate(null)
                currentOnResult.value(FilePickerResult.Cancelled)
              }
            }

          picker.retainFilePickerDelegate(delegate)
          picker.delegate = delegate
          presenter.presentViewController(picker, animated = true, completion = null)
        }
      }
    }
  }
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
  var controller = UIApplication.sharedApplication.keyWindow?.rootViewController ?: return null

  while (controller.presentedViewController != null) {
    controller = controller.presentedViewController!!
  }

  return controller
}

private fun NSData.toByteArray(): ByteArray {
  if (length == 0uL) {
    return ByteArray(0)
  }
  val byteArray = ByteArray(length.toInt())
  byteArray.usePinned { pinned -> memcpy(pinned.addressOf(0), bytes, length) }
  return byteArray
}

private fun UIImage.pixelWidth(): Int = size.useContents { (width * scale).roundToInt() }

private fun UIImage.pixelHeight(): Int = size.useContents { (height * scale).roundToInt() }
