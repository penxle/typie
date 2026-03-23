package co.typie.screen.stats

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.awt.Image
import java.awt.Toolkit
import java.awt.datatransfer.DataFlavor
import java.awt.datatransfer.Transferable
import java.io.ByteArrayInputStream
import java.io.File
import javax.imageio.ImageIO

@Composable
actual fun rememberStatsImageExporter(): StatsImageExporter {
  return remember { JvmStatsImageExporter() }
}

private class JvmStatsImageExporter : StatsImageExporter {
  override suspend fun copyPng(
    bytes: ByteArray,
    suggestedName: String,
  ): Boolean = withContext(Dispatchers.IO) {
    runCatching {
      val image = ImageIO.read(ByteArrayInputStream(bytes)) ?: return@withContext false
      Toolkit.getDefaultToolkit().systemClipboard.setContents(ImageTransferable(image), null)
      true
    }.getOrDefault(false)
  }

  override suspend fun savePng(
    bytes: ByteArray,
    suggestedName: String,
  ): StatsImageSaveResult = withContext(Dispatchers.IO) {
    runCatching {
      val image = ImageIO.read(ByteArrayInputStream(bytes)) ?: return@withContext StatsImageSaveResult.Error
      val picturesDirectory = File(System.getProperty("user.home"), "Pictures").apply { mkdirs() }
      val file = uniqueFile(picturesDirectory, ensureJvmPngFilename(suggestedName))
      ImageIO.write(image, "png", file)
      StatsImageSaveResult.Success
    }.getOrElse {
      StatsImageSaveResult.Error
    }
  }
}

private class ImageTransferable(
  private val image: Image,
) : Transferable {
  override fun getTransferDataFlavors(): Array<DataFlavor> = arrayOf(DataFlavor.imageFlavor)

  override fun isDataFlavorSupported(flavor: DataFlavor): Boolean {
    return flavor == DataFlavor.imageFlavor
  }

  override fun getTransferData(flavor: DataFlavor): Any {
    require(isDataFlavorSupported(flavor)) { "Unsupported data flavor: $flavor" }
    return image
  }
}

private fun ensureJvmPngFilename(name: String): String {
  return if (name.endsWith(".png", ignoreCase = true)) name else "$name.png"
}

private fun uniqueFile(
  directory: File,
  filename: String,
): File {
  val baseName = filename.removeSuffix(".png")
  var candidate = File(directory, filename)
  var index = 1

  while (candidate.exists()) {
    candidate = File(directory, "$baseName-$index.png")
    index += 1
  }

  return candidate
}
