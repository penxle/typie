package co.typie.editortexture

import android.opengl.*
import android.view.Surface
import io.flutter.embedding.engine.plugins.FlutterPlugin
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import io.flutter.view.TextureRegistry
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.util.concurrent.locks.ReentrantLock

class EditorTexturePlugin : FlutterPlugin, MethodChannel.MethodCallHandler {
  private lateinit var channel: MethodChannel
  private lateinit var textureRegistry: TextureRegistry
  private val textures = mutableMapOf<Long, EditorTexture>()

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

    val entry = textureRegistry.createSurfaceTexture()
    val texture = EditorTexture(entry, width, height)
    val textureId = entry.id()
    textures[textureId] = texture

    result.success(textureId)
  }

  private fun handleRender(call: MethodCall, result: MethodChannel.Result) {
    val textureId = call.argument<Number>("textureId")?.toLong() ?: run {
      result.error("INVALID_ARGS", "Missing textureId", null)
      return
    }
    val editorPtr = call.argument<Number>("editorPtr")?.toLong() ?: run {
      result.error("INVALID_ARGS", "Missing editorPtr", null)
      return
    }
    val pageIndex = call.argument<Int>("pageIndex") ?: run {
      result.error("INVALID_ARGS", "Missing pageIndex", null)
      return
    }
    val width = call.argument<Int>("width") ?: run {
      result.error("INVALID_ARGS", "Missing width", null)
      return
    }
    val height = call.argument<Int>("height") ?: run {
      result.error("INVALID_ARGS", "Missing height", null)
      return
    }

    val texture = textures[textureId] ?: run {
      result.error("NOT_FOUND", "Texture not found", null)
      return
    }

    if (texture.render(editorPtr, pageIndex, width, height)) {
      result.success(true)
    } else {
      result.error("RENDER_FAILED", "Render failed", null)
    }
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
  private val entry: TextureRegistry.SurfaceTextureEntry,
  initialWidth: Int,
  initialHeight: Int
) {
  private var eglDisplay: EGLDisplay? = null
  private var eglContext: EGLContext? = null
  private var eglSurface: EGLSurface? = null
  private var glTexture: Int = 0
  private var surface: Surface? = null
  var currentWidth = initialWidth
    private set
  var currentHeight = initialHeight
    private set
  private var program: Int = 0
  private var vertexBuffer: java.nio.FloatBuffer? = null

  private var frontBuffer: ByteBuffer? = null
  private var backBuffer: ByteBuffer? = null
  private var bufferCapacity: Int = 0
  private val bufferLock = ReentrantLock()

  init {
    initEGL()
    initShaders()
    createBuffer(initialWidth, initialHeight)
  }

  private fun initEGL() {
    eglDisplay = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY)
    if (eglDisplay == EGL14.EGL_NO_DISPLAY) {
      throw RuntimeException("Unable to get EGL display")
    }

    val version = IntArray(2)
    if (!EGL14.eglInitialize(eglDisplay, version, 0, version, 1)) {
      throw RuntimeException("Unable to initialize EGL")
    }

    val configAttribs = intArrayOf(
      EGL14.EGL_RED_SIZE, 8,
      EGL14.EGL_GREEN_SIZE, 8,
      EGL14.EGL_BLUE_SIZE, 8,
      EGL14.EGL_ALPHA_SIZE, 8,
      EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT,
      EGL14.EGL_NONE
    )

    val configs = arrayOfNulls<EGLConfig>(1)
    val numConfigs = IntArray(1)
    if (!EGL14.eglChooseConfig(eglDisplay, configAttribs, 0, configs, 0, 1, numConfigs, 0)) {
      throw RuntimeException("Unable to choose EGL config")
    }

    val contextAttribs = intArrayOf(
      EGL14.EGL_CONTEXT_CLIENT_VERSION, 2,
      EGL14.EGL_NONE
    )
    eglContext = EGL14.eglCreateContext(eglDisplay, configs[0], EGL14.EGL_NO_CONTEXT, contextAttribs, 0)

    val surfaceTexture = entry.surfaceTexture()
    surfaceTexture.setDefaultBufferSize(currentWidth, currentHeight)
    surface = Surface(surfaceTexture)

    val surfaceAttribs = intArrayOf(EGL14.EGL_NONE)
    eglSurface = EGL14.eglCreateWindowSurface(eglDisplay, configs[0], surface, surfaceAttribs, 0)

    EGL14.eglMakeCurrent(eglDisplay, eglSurface, eglSurface, eglContext)

    val textures = IntArray(1)
    GLES20.glGenTextures(1, textures, 0)
    glTexture = textures[0]

    GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, glTexture)
    GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_NEAREST)
    GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_MAG_FILTER, GLES20.GL_NEAREST)
    GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_WRAP_S, GLES20.GL_CLAMP_TO_EDGE)
    GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, GLES20.GL_TEXTURE_WRAP_T, GLES20.GL_CLAMP_TO_EDGE)
  }

  private fun initShaders() {
    val vertexShaderCode = """
      attribute vec4 aPosition;
      attribute vec2 aTexCoord;
      varying vec2 vTexCoord;
      void main() {
        gl_Position = aPosition;
        vTexCoord = aTexCoord;
      }
    """.trimIndent()

    val fragmentShaderCode = """
      precision mediump float;
      varying vec2 vTexCoord;
      uniform sampler2D uTexture;
      void main() {
        gl_FragColor = texture2D(uTexture, vTexCoord);
      }
    """.trimIndent()

    val vertexShader = loadShader(GLES20.GL_VERTEX_SHADER, vertexShaderCode)
    val fragmentShader = loadShader(GLES20.GL_FRAGMENT_SHADER, fragmentShaderCode)

    program = GLES20.glCreateProgram()
    GLES20.glAttachShader(program, vertexShader)
    GLES20.glAttachShader(program, fragmentShader)
    GLES20.glLinkProgram(program)

    GLES20.glDeleteShader(vertexShader)
    GLES20.glDeleteShader(fragmentShader)

    val vertices = floatArrayOf(
      -1f, -1f, 0f, 1f,
       1f, -1f, 1f, 1f,
      -1f,  1f, 0f, 0f,
       1f,  1f, 1f, 0f
    )

    vertexBuffer = ByteBuffer.allocateDirect(vertices.size * 4)
      .order(ByteOrder.nativeOrder())
      .asFloatBuffer()
      .put(vertices)
    vertexBuffer?.position(0)
  }

  private fun createBuffer(width: Int, height: Int) {
    val size = width * height * 4
    if (size > bufferCapacity) {
      frontBuffer = ByteBuffer.allocateDirect(size).order(ByteOrder.nativeOrder())
      backBuffer = ByteBuffer.allocateDirect(size).order(ByteOrder.nativeOrder())
      bufferCapacity = size
    }
    currentWidth = width
    currentHeight = height
    entry.surfaceTexture().setDefaultBufferSize(width, height)
  }

  fun render(editorPtr: Long, pageIndex: Int, width: Int, height: Int): Boolean {
    if (!bufferLock.tryLock()) return false

    try {
      if (width != currentWidth || height != currentHeight) {
        createBuffer(width, height)
      }

      val buffer = backBuffer ?: return false

      val ptr = nativeGetDirectBufferAddress(buffer)
      if (ptr == 0L) {
        return false
      }

      val stride = currentWidth * 4L
      val result = nativeRenderPageTo(editorPtr, pageIndex.toLong(), ptr, stride, currentHeight.toLong(), PIXEL_FORMAT_RGBA)

      val temp = frontBuffer
      frontBuffer = backBuffer
      backBuffer = temp

      frontBuffer?.position(0)

      EGL14.eglMakeCurrent(eglDisplay, eglSurface, eglSurface, eglContext)

      GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, glTexture)
      GLES20.glTexImage2D(
        GLES20.GL_TEXTURE_2D, 0, GLES20.GL_RGBA,
        currentWidth, currentHeight, 0, GLES20.GL_RGBA, GLES20.GL_UNSIGNED_BYTE, frontBuffer
      )

      GLES20.glViewport(0, 0, currentWidth, currentHeight)
      GLES20.glClearColor(0f, 0f, 0f, 0f)
      GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)

      drawTexture()

      EGL14.eglSwapBuffers(eglDisplay, eglSurface)

      return result == 0L
    } finally {
      bufferLock.unlock()
    }
  }

  private fun drawTexture() {
    GLES20.glUseProgram(program)

    vertexBuffer?.position(0)
    val positionHandle = GLES20.glGetAttribLocation(program, "aPosition")
    GLES20.glEnableVertexAttribArray(positionHandle)
    GLES20.glVertexAttribPointer(positionHandle, 2, GLES20.GL_FLOAT, false, 16, vertexBuffer)

    vertexBuffer?.position(2)
    val texCoordHandle = GLES20.glGetAttribLocation(program, "aTexCoord")
    GLES20.glEnableVertexAttribArray(texCoordHandle)
    GLES20.glVertexAttribPointer(texCoordHandle, 2, GLES20.GL_FLOAT, false, 16, vertexBuffer)

    val textureHandle = GLES20.glGetUniformLocation(program, "uTexture")
    GLES20.glActiveTexture(GLES20.GL_TEXTURE0)
    GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, glTexture)
    GLES20.glUniform1i(textureHandle, 0)

    GLES20.glEnable(GLES20.GL_BLEND)
    GLES20.glBlendFunc(GLES20.GL_ONE, GLES20.GL_ONE_MINUS_SRC_ALPHA)

    GLES20.glDrawArrays(GLES20.GL_TRIANGLE_STRIP, 0, 4)

    GLES20.glDisableVertexAttribArray(positionHandle)
    GLES20.glDisableVertexAttribArray(texCoordHandle)
  }

  private fun loadShader(type: Int, shaderCode: String): Int {
    val shader = GLES20.glCreateShader(type)
    GLES20.glShaderSource(shader, shaderCode)
    GLES20.glCompileShader(shader)
    return shader
  }

  private external fun nativeGetDirectBufferAddress(buffer: ByteBuffer): Long
  private external fun nativeRenderPageTo(editorPtr: Long, pageIndex: Long, dstPtr: Long, dstStride: Long, dstHeight: Long, format: Long): Long

  fun dispose() {
    bufferLock.lock()
    try {
      EGL14.eglMakeCurrent(eglDisplay, eglSurface, eglSurface, eglContext)

      if (program != 0) {
        GLES20.glDeleteProgram(program)
        program = 0
      }
      if (glTexture != 0) {
        GLES20.glDeleteTextures(1, intArrayOf(glTexture), 0)
        glTexture = 0
      }

      EGL14.eglMakeCurrent(eglDisplay, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_SURFACE, EGL14.EGL_NO_CONTEXT)

      eglSurface?.let {
        EGL14.eglDestroySurface(eglDisplay, it)
        eglSurface = null
      }
      eglContext?.let {
        EGL14.eglDestroyContext(eglDisplay, it)
        eglContext = null
      }
      surface?.release()
      surface = null
      entry.release()
      frontBuffer = null
      backBuffer = null
    } finally {
      bufferLock.unlock()
    }
  }

  companion object {
    private const val PIXEL_FORMAT_RGBA = 0L

    init {
      System.loadLibrary("editor")
    }
  }
}
