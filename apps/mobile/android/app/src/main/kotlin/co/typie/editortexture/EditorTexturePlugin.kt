package co.typie.editortexture

import android.graphics.PixelFormat
import android.hardware.HardwareBuffer
import android.media.ImageReader
import android.media.ImageWriter
import android.os.Handler
import android.os.Looper
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry
import java.nio.ByteBuffer
import java.util.concurrent.Executors
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.locks.ReentrantLock

class EditorTexturePlugin : FlutterPlugin, MethodChannel.MethodCallHandler {
  private lateinit var channel: MethodChannel
  private lateinit var textureRegistry: TextureRegistry
  private val textures = mutableMapOf<Long, EditorTexture>()
  private val texturesLock = Any()
  private val mainHandler = Handler(Looper.getMainLooper())
  private val renderExecutor = Executors.newSingleThreadExecutor()

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
    renderExecutor.shutdownNow()
    val toDispose = synchronized(texturesLock) {
      val values = textures.values.toList()
      textures.clear()
      values
    }
    toDispose.forEach { it.dispose() }
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
      replyError(result, "INVALID_ARGS", "Missing width")
      return
    }
    val height = call.argument<Int>("height") ?: run {
      replyError(result, "INVALID_ARGS", "Missing height")
      return
    }

    postOnMainOrReplyError(result, "CREATE_FAILED") {
      val toEvict = mutableListOf<EditorTexture>()
      synchronized(texturesLock) {
        while (textures.size >= MAX_TEXTURES) {
          val oldestId = textures.keys.minOrNull() ?: break
          val removed = textures.remove(oldestId)
          if (removed != null) {
            toEvict.add(removed)
          }
        }
      }
      toEvict.forEach { it.dispose() }

      val entry = textureRegistry.createImageTexture()
      val texture = EditorTexture(entry, width, height, mainHandler)
      val textureId = entry.id()
      synchronized(texturesLock) {
        textures[textureId] = texture
      }

      result.success(textureId)
    }
  }

  @Suppress("UNCHECKED_CAST")
  private fun handleRender(call: MethodCall, result: MethodChannel.Result) {
    val items = call.argument<List<Map<String, Any>>>("items") ?: run {
      replyError(result, "INVALID_ARGS", "Missing items")
      return
    }

    try {
      renderExecutor.execute {
        try {
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

            val texture = synchronized(texturesLock) {
              textures[textureId]
            }
            val didRender = texture?.render(editorPtr, pageIndex, width, height) ?: false
            results.add(didRender)
          }

          replySuccess(result, results)
        } catch (t: Throwable) {
          replyError(result, "RENDER_FAILED", t.message ?: "Render failed")
        }
      }
    } catch (_: RejectedExecutionException) {
      replySuccess(result, List(items.size) { false })
    }
  }

  private fun handleDispose(call: MethodCall, result: MethodChannel.Result) {
    val textureId = call.argument<Number>("textureId")?.toLong() ?: run {
      replyError(result, "INVALID_ARGS", "Missing textureId")
      return
    }

    postOnMainOrReplyError(result, "DISPOSE_FAILED") {
      val texture = synchronized(texturesLock) {
        textures.remove(textureId)
      }
      texture?.dispose()
      result.success(null)
    }
  }

  private fun replySuccess(result: MethodChannel.Result, value: Any?) {
    if (Looper.myLooper() == Looper.getMainLooper()) {
      result.success(value)
      return
    }
    if (!mainHandler.post { result.success(value) }) {
      result.success(value)
    }
  }

  private fun replyError(result: MethodChannel.Result, code: String, message: String) {
    if (Looper.myLooper() == Looper.getMainLooper()) {
      result.error(code, message, null)
      return
    }
    if (!mainHandler.post { result.error(code, message, null) }) {
      result.error(code, message, null)
    }
  }

  private inline fun postOnMainOrReplyError(
    result: MethodChannel.Result,
    errorCode: String,
    crossinline block: () -> Unit
  ) {
    val task = Runnable {
      try {
        block()
      } catch (t: Throwable) {
        result.error(errorCode, t.message ?: errorCode, null)
      }
    }

    if (Looper.myLooper() == Looper.getMainLooper()) {
      task.run()
      return
    }
    if (!mainHandler.post(task)) {
      result.error(errorCode, "Failed to post work to main thread", null)
    }
  }
}

class EditorTexture(
  private val entry: TextureRegistry.ImageTextureEntry,
  initialWidth: Int,
  initialHeight: Int,
  private val mainHandler: Handler
) {
  private var imageReader: ImageReader? = null
  private var imageWriter: ImageWriter? = null
  private var currentWidth = initialWidth
  private var currentHeight = initialHeight
  private val bufferLock = ReentrantLock()
  @Volatile
  private var disposed = false

  init {
    check(createPipeline(initialWidth, initialHeight)) { "Failed to create initial image pipeline" }
  }

  private fun createPipeline(width: Int, height: Int): Boolean {
    if (!releasePipeline(clearTexture = true, waitForClear = true)) {
      return false
    }

    val reader = ImageReader.newInstance(
      width, height, PixelFormat.RGBA_8888, 4,
      HardwareBuffer.USAGE_CPU_WRITE_OFTEN or HardwareBuffer.USAGE_GPU_SAMPLED_IMAGE
    )
    val writer = ImageWriter.newInstance(reader.surface, 2)

    imageReader = reader
    imageWriter = writer
    currentWidth = width
    currentHeight = height
    return true
  }

  private fun releasePipeline(clearTexture: Boolean = true, waitForClear: Boolean = false): Boolean {
    val hasPipeline = imageReader != null || imageWriter != null
    if (clearTexture && hasPipeline && !pushImageOnMain(null, wait = waitForClear)) {
      return false
    }
    imageWriter?.close()
    imageWriter = null
    imageReader?.close()
    imageReader = null
    return true
  }

  private fun closeImageSafely(image: android.media.Image?) {
    if (image == null) {
      return
    }
    try {
      image.close()
    } catch (_: IllegalStateException) {
      // May already be closed by Flutter internals.
    }
  }

  private fun pushImageOnMain(image: android.media.Image?, wait: Boolean): Boolean {
    var pushed = false
    val task = Runnable {
      if (disposed) {
        closeImageSafely(image)
        return@Runnable
      }
      try {
        entry.pushImage(image)
        pushed = true
      } catch (_: IllegalStateException) {
        closeImageSafely(image)
      }
    }

    if (Looper.myLooper() == Looper.getMainLooper()) {
      task.run()
      return pushed
    }

    if (!wait) {
      if (!mainHandler.post(task)) {
        closeImageSafely(image)
      }
      return false
    }

    val latch = java.util.concurrent.CountDownLatch(1)
    val aborted = java.util.concurrent.atomic.AtomicBoolean(false)
    if (!mainHandler.post({
      try {
        if (aborted.get()) {
          closeImageSafely(image)
          return@post
        }
        task.run()
      } finally {
        latch.countDown()
      }
    })) {
      closeImageSafely(image)
      return false
    }
    try {
      if (!latch.await(MAIN_THREAD_PUSH_TIMEOUT_MS, java.util.concurrent.TimeUnit.MILLISECONDS)) {
        aborted.set(true)
        closeImageSafely(image)
        return false
      }
    } catch (_: InterruptedException) {
      aborted.set(true)
      Thread.currentThread().interrupt()
      closeImageSafely(image)
      return false
    }
    return pushed
  }

  fun render(editorPtr: Long, pageIndex: Int, width: Int, height: Int): Boolean {
    var outputImage: android.media.Image? = null

    if (!bufferLock.tryLock()) return false

    try {
      if (disposed) {
        return false
      }

      if (width != currentWidth || height != currentHeight) {
        if (!createPipeline(width, height)) {
          return false
        }
      }

      val writer = imageWriter ?: return false
      val reader = imageReader ?: return false

      val inputImage = try {
        writer.dequeueInputImage()
      } catch (_: IllegalStateException) {
        return false
      }

      val plane = inputImage.planes[0]
      val buffer = plane.buffer
      val ptr = nativeGetDirectBufferAddress(buffer)
      if (ptr == 0L) {
        closeImageSafely(inputImage)
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
        closeImageSafely(inputImage)
        return false
      }

      try {
        writer.queueInputImage(inputImage)
      } catch (_: IllegalStateException) {
        closeImageSafely(inputImage)
        return false
      }

      outputImage = try {
        reader.acquireLatestImage()
      } catch (_: IllegalStateException) {
        return false
      } ?: return false
    } finally {
      bufferLock.unlock()
    }

    return pushImageOnMain(outputImage, wait = true)
  }

  fun dispose() {
    bufferLock.lock()
    try {
      if (disposed) {
        return
      }
      pushImageOnMain(null, wait = true)
      disposed = true
      releasePipeline(clearTexture = false, waitForClear = false)
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
    private const val MAIN_THREAD_PUSH_TIMEOUT_MS = 250L

    init {
      System.loadLibrary("editor")
    }
  }
}
