package co.typie.media

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
import platform.UIKit.UIApplication
import platform.UIKit.UIImage
import platform.UIKit.UIImageJPEGRepresentation
import platform.UIKit.UIImagePickerController
import platform.UIKit.UIImagePickerControllerDelegateProtocol
import platform.UIKit.UIImagePickerControllerOriginalImage
import platform.UIKit.UIImagePickerControllerSourceType
import platform.UIKit.UINavigationControllerDelegateProtocol
import platform.UIKit.UIViewController
import platform.darwin.NSObject
import platform.posix.memcpy

@Composable
actual fun rememberImagePicker(
  onResult: (PickedImage?) -> Unit,
): () -> Unit {
  val currentOnResult = rememberUpdatedState(onResult)
  var delegateHolder by remember { mutableStateOf<NSObject?>(null) }

  return remember {
    {
      val presenter = topViewController() ?: run {
        currentOnResult.value(null)
        return@remember
      }

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
            PickedImage(
              bytes = data.toByteArray(),
              filename = pickedImageFilename(
                originalFilename = null,
                mimeType = "image/jpeg",
              ),
              mimeType = "image/jpeg",
            ),
          )
        }
      }

      delegateHolder = delegate
      picker.delegate = delegate
      presenter.presentViewController(picker, animated = true, completion = null)
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

@OptIn(ExperimentalForeignApi::class)
private fun NSData.toByteArray(): ByteArray {
  val byteArray = ByteArray(length.toInt())
  byteArray.usePinned { pinned ->
    memcpy(pinned.addressOf(0), bytes, length)
  }
  return byteArray
}
