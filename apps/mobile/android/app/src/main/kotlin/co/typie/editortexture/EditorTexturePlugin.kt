package co.typie.editortexture

import android.graphics.PixelFormat
import android.hardware.HardwareBuffer
import android.media.ImageReader
import android.media.ImageWriter
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry
import java.nio.ByteBuffer
import java.util.concurrent.locks.ReentrantLock

class EditorTexturePlugin : FlutterPlugin, MethodChannel.MethodCallHandler {
  private lateinit var channel: MethodChannel
  private lateinit var textureRegistry: TextureRegistry
  private val textures = mutableMapOf<Long, EditorTexture>()

  companion object {
    private const val MAX_TEXTURES = 10
  }

  override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
    channel = MethodChannel(binding.binaryMessenger, "co.typie.editor_texture")
    channel.setMethodCallHandler(this)
    textureRegistry = binding.textureRegistry
  }

  override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
    channel.setMethodCallHandler(null)
    textures.values.forEach { it.dispose() }
    textures.clear()
  }

  override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
    when (call.method) {
      "create" -> handleCreate(call, result)
      "render" -> handleRender(call, result)
      "dispose" -> handleDispose(call, result)
      else -> result.notImplemented()
    }
  }

  private fun handleCreate(call: MethodCall, result: MethodChannel.Result) {
    val width = call.argument<Int>("width") ?: run {
      result.error("INVALID_ARGS", "Missing width", null)
      return
    }
    val height = call.argument<Int>("height") ?: run {
      result.error("INVALID_ARGS", "Missing height", null)
      return
    }

    while (textures.size >= MAX_TEXTURES) {
      val oldestId = textures.keys.minOrNull() ?: break
      textures.remove(oldestId)?.dispose()
    }

    val entry = textureRegistry.createImageTexture()
    val texture = EditorTexture(entry, width, height)
    val textureId = entry.id()
    textures[textureId] = texture

    result.success(textureId)
  }

  @Suppress("UNCHECKED_CAST")
  private fun handleRender(call: MethodCall, result: MethodChannel.Result) {
    val items = call.argument<List<Map<String, Any>>>("items") ?: run {
      result.error("INVALID_ARGS", "Missing items", null)
      return
    }

    val results = mutableListOf<Boolean>()

    for (item in items) {
      val textureId = (item["textureId"] as? Number)?.toLong()
      val editorPtr = (item["editorPtr"] as? Number)?.toLong()
      val pageIndex = item["pageIndex"] as? Int
      val width = item["width"] as? Int
      val height = item["height"] as? Int

      if (textureId == null || editorPtr == null || pageIndex == null || width == null || height == null) {
        results.add(false)
        continue
      }

      val texture = textures[textureId]
      val didRender = texture?.render(editorPtr, pageIndex, width, height) ?: false
      results.add(didRender)
    }

    result.success(results)
  }

  private fun handleDispose(call: MethodCall, result: MethodChannel.Result) {
    val textureId = call.argument<Number>("textureId")?.toLong() ?: run {
      result.error("INVALID_ARGS", "Missing textureId", null)
      return
    }

    textures.remove(textureId)?.dispose()
    result.success(null)
  }
}

class EditorTexture(
  private val entry: TextureRegistry.ImageTextureEntry,
  initialWidth: Int,
  initialHeight: Int
) {
  private var imageReader: ImageReader? = null
  private var imageWriter: ImageWriter? = null
  private var currentWidth = initialWidth
  private var currentHeight = initialHeight
  private val bufferLock = ReentrantLock()
  private var prevPrevImage: android.media.Image? = null
  private var prevImage: android.media.Image? = null

  init {
    createPipeline(initialWidth, initialHeight)
  }

  private fun createPipeline(width: Int, height: Int) {
    releasePipeline()

    val reader = ImageReader.newInstance(
      width, height, PixelFormat.RGBA_8888, 4,
      HardwareBuffer.USAGE_CPU_WRITE_OFTEN or HardwareBuffer.USAGE_GPU_SAMPLED_IMAGE
    )
    val writer = ImageWriter.newInstance(reader.surface, 2)

    imageReader = reader
    imageWriter = writer
    currentWidth = width
    currentHeight = height
  }

  private fun releasePipeline() {
    entry.pushImage(null)
    safeClose(prevPrevImage)
    prevPrevImage = null
    safeClose(prevImage)
    prevImage = null
    imageWriter?.close()
    imageWriter = null
    imageReader?.close()
    imageReader = null
  }

  private fun safeClose(image: android.media.Image?) {
    if (image == null) {
      return
    }
    try {
      image.close()
    } catch (_: IllegalStateException) {
      // May already be closed by Flutter internals.
    }
  }

  fun render(editorPtr: Long, pageIndex: Int, width: Int, height: Int): Boolean {
    if (!bufferLock.tryLock()) return false

    try {
      if (width != currentWidth || height != currentHeight) {
        createPipeline(width, height)
      }

      val writer = imageWriter ?: return false
      val reader = imageReader ?: return false

      safeClose(prevPrevImage)
      prevPrevImage = prevImage
      prevImage = null

      val inputImage = try {
        writer.dequeueInputImage()
      } catch (_: IllegalStateException) {
        return false
      }

      val plane = inputImage.planes[0]
      val buffer = plane.buffer
      val ptr = nativeGetDirectBufferAddress(buffer)
      if (ptr == 0L) {
        inputImage.close()
        return false
      }

      val result = nativeRenderPageTo(
        editorPtr,
        pageIndex.toLong(),
        ptr,
        plane.rowStride.toLong(),
        currentWidth.toLong(),
        currentHeight.toLong(),
        PIXEL_FORMAT_RGBA
      )

      if (result != 0L) {
        inputImage.close()
        return false
      }

      try {
        writer.queueInputImage(inputImage)
      } catch (_: IllegalStateException) {
        inputImage.close()
        return false
      }

      val outputImage = try {
        reader.acquireLatestImage()
      } catch (_: IllegalStateException) {
        return false
      } ?: return false
      entry.pushImage(outputImage)
      prevImage = outputImage

      return true
    } finally {
      bufferLock.unlock()
    }
  }

  fun dispose() {
    bufferLock.lock()
    try {
      releasePipeline()
      entry.release()
    } finally {
      bufferLock.unlock()
    }
  }

  private external fun nativeGetDirectBufferAddress(buffer: ByteBuffer): Long
  private external fun nativeRenderPageTo(
    editorPtr: Long,
    pageIndex: Long,
    dstPtr: Long,
    dstStride: Long,
    dstWidth: Long,
    dstHeight: Long,
    format: Long
  ): Long

  companion object {
    private const val PIXEL_FORMAT_RGBA = 0L

    init {
      System.loadLibrary("editor")
    }
  }
}
