package co.typie.platform

import co.typie.di.PlatformContext
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single
import java.awt.Image
import java.awt.Toolkit
import java.awt.datatransfer.DataFlavor
import java.awt.datatransfer.StringSelection
import java.awt.datatransfer.Transferable
import java.io.ByteArrayInputStream
import java.io.File
import javax.imageio.ImageIO

@Module
actual class PlatformServiceModule {
  @Single
  actual fun clipboard(ctx: PlatformContext): Clipboard = DesktopClipboard()

  @Single
  actual fun deviceInfo(ctx: PlatformContext): DeviceInfo = DesktopDeviceInfo()

  @Single
  actual fun fileSystem(ctx: PlatformContext): FileSystem = DesktopFileSystem()

  @Single
  actual fun purchaseService(ctx: PlatformContext): PurchaseService = DesktopPurchaseService()

  @Single
  actual fun share(ctx: PlatformContext): Share = DesktopShare()
}

private class DesktopDeviceInfo : DeviceInfo {
  override suspend fun snapshot(): DeviceInfoSnapshot = withContext(Dispatchers.IO) {
    val osName = System.getProperty("os.name")?.takeIf { it.isNotBlank() } ?: "Desktop"
    val osVersion = System.getProperty("os.version")?.takeIf { it.isNotBlank() } ?: "unknown"
    val appVersion = System.getProperty("app.version")?.takeIf { it.isNotBlank() } ?: "dev"
    val deviceName = sequenceOf(
      System.getenv("COMPUTERNAME"),
      System.getenv("HOSTNAME"),
      System.getProperty("user.name"),
    ).firstOrNull { !it.isNullOrBlank() }

    DeviceInfoSnapshot(
      platform = osName,
      osVersion = osVersion,
      appVersion = appVersion,
      deviceName = deviceName,
    )
  }
}

private class DesktopClipboard : Clipboard {
  override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean = withContext(Dispatchers.IO) {
    runCatching {
      if (mimeType.startsWith("image/")) {
        val image = ImageIO.read(ByteArrayInputStream(bytes)) ?: return@withContext false
        Toolkit.getDefaultToolkit().systemClipboard.setContents(ImageTransferable(image), null)
      } else {
        return@withContext false
      }
      true
    }.getOrDefault(false)
  }

  override suspend fun copy(text: String, mimeType: String): Boolean = withContext(Dispatchers.IO) {
    runCatching {
      Toolkit.getDefaultToolkit().systemClipboard.setContents(StringSelection(text), null)
      true
    }.getOrDefault(false)
  }
}

private class ImageTransferable(
  private val image: Image,
) : Transferable {
  override fun getTransferDataFlavors(): Array<DataFlavor> = arrayOf(DataFlavor.imageFlavor)
  override fun isDataFlavorSupported(flavor: DataFlavor): Boolean = flavor == DataFlavor.imageFlavor
  override fun getTransferData(flavor: DataFlavor): Any {
    require(isDataFlavorSupported(flavor)) { "Unsupported data flavor: $flavor" }
    return image
  }
}

private class DesktopFileSystem : FileSystem {
  override suspend fun save(
    bytes: ByteArray,
    name: String,
    location: FileSystemSaveLocation,
  ): FileSystemSaveResult = withContext(Dispatchers.IO) {
    runCatching {
      val directory = when (location) {
        FileSystemSaveLocation.Gallery -> File(System.getProperty("user.home"), "Pictures")
        FileSystemSaveLocation.Files -> File(System.getProperty("user.home"), "Downloads")
      }
      directory.mkdirs()

      val file = uniqueFile(directory, name)
      file.writeBytes(bytes)
      FileSystemSaveResult.Success
    }.getOrElse {
      FileSystemSaveResult.Error
    }
  }
}

private class DesktopShare : Share {
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
