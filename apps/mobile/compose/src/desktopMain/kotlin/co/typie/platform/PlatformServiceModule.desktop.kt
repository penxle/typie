package co.typie.platform

import java.awt.Image
import java.awt.Toolkit
import java.awt.datatransfer.DataFlavor
import java.awt.datatransfer.StringSelection
import java.awt.datatransfer.Transferable
import java.awt.image.BufferedImage
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import javax.imageio.ImageIO
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.io.asSource
import kotlinx.io.buffered

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

  override suspend fun paste(): IncomingContentCandidates? =
    withContext(Dispatchers.IO) {
      runCatching {
          val contents =
            Toolkit.getDefaultToolkit().systemClipboard.getContents(null) ?: return@runCatching null
          contents.readIncomingContentCandidates()
        }
        .getOrNull()
    }
}

internal fun Transferable.readString(flavor: DataFlavor): String? {
  if (!isDataFlavorSupported(flavor)) return null
  return runCatching {
      when (val data = getTransferData(flavor)) {
        is String -> data
        is java.io.Reader -> data.use(java.io.Reader::readText)
        else -> null
      }
    }
    .getOrNull()
}

internal fun Transferable.readHtml(): String? =
  listOf(DataFlavor.allHtmlFlavor, DataFlavor.fragmentHtmlFlavor, DataFlavor.selectionHtmlFlavor)
    .firstNotNullOfOrNull { flavor -> readString(flavor)?.takeIf(String::isNotEmpty) }

internal suspend fun Transferable.readIncomingContentCandidates(): IncomingContentCandidates? =
  materializeIncomingContentCandidates(
    html = readHtml(),
    text = readString(DataFlavor.stringFlavor),
    loadItems = ::readAttachmentItems,
  )

internal fun Transferable.readAttachmentItems(): LoadedIncomingContentItems {
  if (isDataFlavorSupported(DataFlavor.javaFileListFlavor)) {
    val transferred = runCatching { getTransferData(DataFlavor.javaFileListFlavor) as? List<*> }
    if (transferred.isFailure) {
      return LoadedIncomingContentItems(emptyList(), unreadableItemCount = 1)
    }
    val transferredFiles =
      transferred.getOrNull()
        ?: return LoadedIncomingContentItems(emptyList(), unreadableItemCount = 1)
    val files = transferredFiles.filterIsInstance<File>()
    if (transferredFiles.isNotEmpty()) {
      val outcomes = files.map { file ->
        runCatching {
          check(file.isFile && file.canRead()) { "Clipboard file is unreadable" }
          val providerMimeType = file.probeContentType()
          val svgMimeType = svgMimeTypeOrNull(file.name, providerMimeType)
          val mimeType = svgMimeType ?: providerMimeType
          val imageSize =
            when {
              svgMimeType != null -> decodeSvgImageSize(file.readBytes())
              mimeType?.substringBefore('/') == "image" ->
                checkNotNull(file.decodeImageOrNull()) { "Clipboard image is unreadable" }
                  .let { it.width to it.height }
              else -> null
            }
          IncomingContentItem(
            kind =
              if (mimeType?.substringBefore('/') == "image") {
                IncomingContentItem.Kind.Image
              } else {
                IncomingContentItem.Kind.File
              },
            file =
              PickedFile(
                filename = file.name,
                mimeType = mimeType,
                size = file.length(),
                previewModel = file,
                imageWidth = imageSize?.first,
                imageHeight = imageSize?.second,
                openSource = { file.inputStream().asSource().buffered() },
              ),
          )
        }
      }
      return LoadedIncomingContentItems(
        items = outcomes.mapNotNull(Result<IncomingContentItem>::getOrNull),
        unreadableItemCount =
          transferredFiles.size - files.size +
            outcomes.count(Result<IncomingContentItem>::isFailure),
      )
    }
  }

  if (!isDataFlavorSupported(DataFlavor.imageFlavor)) return LoadedIncomingContentItems(emptyList())
  return runCatching {
      val image = checkNotNull(getTransferData(DataFlavor.imageFlavor) as? Image)
      val buffered = checkNotNull(image.toBufferedImageOrNull())
      val bytes =
        ByteArrayOutputStream().use { output ->
          check(ImageIO.write(buffered, "png", output))
          output.toByteArray()
        }
      IncomingContentItem(
        kind = IncomingContentItem.Kind.Image,
        file =
          PickedFile(
            filename = "image.png",
            mimeType = "image/png",
            size = bytes.size.toLong(),
            previewModel = buffered,
            imageWidth = buffered.width,
            imageHeight = buffered.height,
            openSource = { ByteArrayInputStream(bytes).asSource().buffered() },
          ),
      )
    }
    .fold(
      onSuccess = { item -> LoadedIncomingContentItems(listOf(item)) },
      onFailure = { LoadedIncomingContentItems(emptyList(), unreadableItemCount = 1) },
    )
}

private fun Image.toBufferedImageOrNull(): BufferedImage? {
  if (this is BufferedImage) return this
  val width = getWidth(null)
  val height = getHeight(null)
  if (width <= 0 || height <= 0) return null
  return BufferedImage(width, height, BufferedImage.TYPE_INT_ARGB).also { buffered ->
    val graphics = buffered.createGraphics()
    try {
      graphics.drawImage(this, 0, 0, null)
    } finally {
      graphics.dispose()
    }
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
      DataFlavor.allHtmlFlavor -> html
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
  override suspend fun share(bytes: ByteArray, mimeType: String, anchor: ShareAnchor?): Boolean =
    false

  override suspend fun share(text: String, anchor: ShareAnchor?): Boolean = false
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
