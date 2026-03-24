@file:OptIn(ExperimentalForeignApi::class, kotlinx.cinterop.BetaInteropApi::class)

package co.typie.platform

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.usePinned
import platform.Foundation.NSData
import platform.Foundation.NSURL
import platform.Foundation.create
import platform.Foundation.dataWithContentsOfURL
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
import platform.UniformTypeIdentifiers.UTTypeImage
import platform.darwin.NSObject
import platform.posix.memcpy

@Composable
actual fun rememberFilePicker(
  onResult: (PlatformFile?) -> Unit,
): (mimeType: String) -> Unit {
  val currentOnResult = rememberUpdatedState(onResult)
  var delegateHolder by remember { mutableStateOf<NSObject?>(null) }

  return remember {
    { mimeType: String ->
      val presenter = topViewController() ?: run {
        currentOnResult.value(null)
        return@remember
      }

      if (mimeType.startsWith("image/")) {
        val picker = UIImagePickerController().apply {
          sourceType = UIImagePickerControllerSourceType.UIImagePickerControllerSourceTypePhotoLibrary
        }

        val delegate = object : NSObject(), UIImagePickerControllerDelegateProtocol, UINavigationControllerDelegateProtocol {
          override fun imagePickerControllerDidCancel(picker: UIImagePickerController) {
            picker.dismissViewControllerAnimated(true, completion = null)
            delegateHolder = null
            currentOnResult.value(null)
          }

          override fun imagePickerController(
            picker: UIImagePickerController,
            didFinishPickingMediaWithInfo: Map<Any?, *>,
          ) {
            val image = didFinishPickingMediaWithInfo[UIImagePickerControllerOriginalImage] as? UIImage
            val data = image?.let { UIImageJPEGRepresentation(it, 0.9) }

            picker.dismissViewControllerAnimated(true, completion = null)
            delegateHolder = null

            if (data == null) {
              currentOnResult.value(null)
              return
            }

            currentOnResult.value(
              PlatformFile(
                bytes = data.toByteArray(),
                filename = pickedFilename(originalFilename = null, mimeType = "image/jpeg"),
                mimeType = "image/jpeg",
              ),
            )
          }
        }

        delegateHolder = delegate
        picker.delegate = delegate
        presenter.presentViewController(picker, animated = true, completion = null)
      } else {
        val types = listOf(UTType.typeWithMIMEType(mimeType) ?: UTTypeImage)
        val picker = UIDocumentPickerViewController(forOpeningContentTypes = types)

        val delegate = object : NSObject(), UIDocumentPickerDelegateProtocol {
          override fun documentPicker(controller: UIDocumentPickerViewController, didPickDocumentsAtURLs: List<*>) {
            delegateHolder = null
            val url = didPickDocumentsAtURLs.firstOrNull() as? NSURL
            if (url == null) {
              currentOnResult.value(null)
              return
            }

            val data = NSData.create(contentsOfURL = url)
            if (data == null) {
              currentOnResult.value(null)
              return
            }

            currentOnResult.value(
              PlatformFile(
                bytes = data.toByteArray(),
                filename = pickedFilename(url.lastPathComponent, mimeType = null),
                mimeType = mimeType,
              ),
            )
          }

          override fun documentPickerWasCancelled(controller: UIDocumentPickerViewController) {
            delegateHolder = null
            currentOnResult.value(null)
          }
        }

        delegateHolder = delegate
        picker.delegate = delegate
        presenter.presentViewController(picker, animated = true, completion = null)
      }
    }
  }
}

private fun topViewController(): UIViewController? {
  var controller = UIApplication.sharedApplication.keyWindow?.rootViewController ?: return null

  while (controller.presentedViewController != null) {
    controller = controller.presentedViewController!!
  }

  return controller
}

private fun NSData.toByteArray(): ByteArray {
  val byteArray = ByteArray(length.toInt())
  byteArray.usePinned { pinned ->
    memcpy(pinned.addressOf(0), bytes, length)
  }
  return byteArray
}
