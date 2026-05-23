package co.typie.platform

import java.awt.Image
import java.awt.Toolkit
import java.awt.datatransfer.DataFlavor
import java.awt.datatransfer.StringSelection
import java.awt.datatransfer.Transferable
import java.io.ByteArrayInputStream
import java.io.File
import java.io.StringReader
import javax.imageio.ImageIO
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

internal class DesktopDeviceInfo : DeviceInfo {
  override fun retrieve(): DeviceInfoData {
    val osName = System.getProperty("os.name")?.takeIf { it.isNotBlank() } ?: "Desktop"
    val osVersion = System.getProperty("os.version")?.takeIf { it.isNotBlank() } ?: "unknown"
    val appVersion = System.getProperty("app.version")?.takeIf { it.isNotBlank() } ?: "dev"

    return DeviceInfoData(
      model = "Desktop",
      osName = osName,
      osVersion = osVersion,
      appVersion = appVersion,
      appBuildNumber = "0",
    )
  }
}

internal class DesktopClipboard : Clipboard {
  override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean =
    withContext(Dispatchers.IO) {
      runCatching {
          if (mimeType.startsWith("image/")) {
            val image = ImageIO.read(ByteArrayInputStream(bytes)) ?: return@withContext false
            Toolkit.getDefaultToolkit().systemClipboard.setContents(ImageTransferable(image), null)
          } else {
            return@withContext false
          }
          true
        }
        .getOrDefault(false)
    }

  override suspend fun copy(text: String, mimeType: String): Boolean =
    withContext(Dispatchers.IO) {
      runCatching {
          Toolkit.getDefaultToolkit().systemClipboard.setContents(StringSelection(text), null)
          true
        }
        .getOrDefault(false)
    }

  override suspend fun copyRichText(html: String, text: String): Boolean =
    withContext(Dispatchers.IO) {
      runCatching {
          Toolkit.getDefaultToolkit()
            .systemClipboard
            .setContents(HtmlTextTransferable(html = html, text = text), null)
          true
        }
        .getOrDefault(false)
    }

  override suspend fun paste(): ClipboardReadPayload? =
    withContext(Dispatchers.IO) {
      runCatching {
          val contents = Toolkit.getDefaultToolkit().systemClipboard.getContents(null)
          val text =
            if (contents.isDataFlavorSupported(DataFlavor.stringFlavor)) {
              contents.getTransferData(DataFlavor.stringFlavor) as? String
            } else {
              null
            } ?: return@runCatching null

          val html =
            if (contents.isDataFlavorSupported(DataFlavor.allHtmlFlavor)) {
              when (val data = contents.getTransferData(DataFlavor.allHtmlFlavor)) {
                is String -> data
                is java.io.Reader -> data.readText()
                else -> null
              }
            } else {
              null
            }
          ClipboardReadPayload(html = html, text = text)
        }
        .getOrNull()
    }
}

internal class ImageTransferable(private val image: Image) : Transferable {
  override fun getTransferDataFlavors(): Array<DataFlavor> = arrayOf(DataFlavor.imageFlavor)

  override fun isDataFlavorSupported(flavor: DataFlavor): Boolean = flavor == DataFlavor.imageFlavor

  override fun getTransferData(flavor: DataFlavor): Any {
    require(isDataFlavorSupported(flavor)) { "Unsupported data flavor: $flavor" }
    return image
  }
}

internal class HtmlTextTransferable(private val html: String, private val text: String) :
  Transferable {
  override fun getTransferDataFlavors(): Array<DataFlavor> =
    arrayOf(DataFlavor.allHtmlFlavor, DataFlavor.stringFlavor)

  override fun isDataFlavorSupported(flavor: DataFlavor): Boolean =
    flavor == DataFlavor.allHtmlFlavor || flavor == DataFlavor.stringFlavor

  override fun getTransferData(flavor: DataFlavor): Any =
    when (flavor) {
      DataFlavor.allHtmlFlavor -> StringReader(html)
      DataFlavor.stringFlavor -> text
      else -> throw IllegalArgumentException("Unsupported data flavor: $flavor")
    }
}

internal class DesktopFileSystem : FileSystem {
  override suspend fun save(
    bytes: ByteArray,
    name: String,
    location: FileSystemSaveLocation,
  ): FileSystemSaveResult =
    withContext(Dispatchers.IO) {
      runCatching {
          val directory =
            when (location) {
              FileSystemSaveLocation.Gallery -> File(System.getProperty("user.home"), "Pictures")
              FileSystemSaveLocation.Files -> File(System.getProperty("user.home"), "Downloads")
            }
          directory.mkdirs()

          val file = uniqueFile(directory, name)
          file.writeBytes(bytes)
          FileSystemSaveResult.Success
        }
        .getOrElse { FileSystemSaveResult.Error }
    }
}

internal class DesktopShare : Share {
  // NOTE: Desktop share flow is not supported yet.
  override suspend fun share(bytes: ByteArray, mimeType: String): Boolean = false

  override suspend fun share(text: String): Boolean = false
}

private fun uniqueFile(directory: File, filename: String): File {
  val dotIndex = filename.lastIndexOf('.')
  val baseName = if (dotIndex > 0) filename.substring(0, dotIndex) else filename
  val extension = if (dotIndex > 0) filename.substring(dotIndex) else ""

  var candidate = File(directory, filename)
  var index = 1

  while (candidate.exists()) {
    candidate = File(directory, "$baseName-$index$extension")
    index += 1
  }

  return candidate
}
