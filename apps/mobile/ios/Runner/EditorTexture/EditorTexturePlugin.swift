import Flutter
import UIKit

class EditorTexturePlugin: NSObject, FlutterPlugin {
  private let textureRegistry: FlutterTextureRegistry
  private var textures: [Int64: EditorTexture] = [:]
  private static let maxTextures = 5

  init(registrar: FlutterPluginRegistrar) {
    self.textureRegistry = registrar.textures()
    super.init()
  }

  static func register(with registrar: FlutterPluginRegistrar) {
    let channel = FlutterMethodChannel(
      name: "co.typie.editor_texture",
      binaryMessenger: registrar.messenger()
    )
    let instance = EditorTexturePlugin(registrar: registrar)
    registrar.addMethodCallDelegate(instance, channel: channel)
  }

  func handle(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    switch call.method {
    case "create":
      handleCreate(call, result: result)
    case "render":
      handleRender(call, result: result)
    case "dispose":
      handleDispose(call, result: result)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  private func handleCreate(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let width = args["width"] as? Int,
          let height = args["height"] as? Int else {
      result(FlutterError(code: "INVALID_ARGS", message: "Missing width or height", details: nil))
      return
    }

    while textures.count >= Self.maxTextures {
      if let oldestId = textures.keys.min() {
        if let oldTexture = textures.removeValue(forKey: oldestId) {
          oldTexture.dispose()
          textureRegistry.unregisterTexture(oldestId)
        }
      } else {
        break
      }
    }

    let texture = EditorTexture(width: width, height: height)
    let textureId = textureRegistry.register(texture)
    textures[textureId] = texture

    result(textureId)
  }

  private func handleRender(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let textureId = args["textureId"] as? Int64,
          let editorPtr = args["editorPtr"] as? Int64,
          let pageIndex = args["pageIndex"] as? Int,
          let width = args["width"] as? Int,
          let height = args["height"] as? Int else {
      result(FlutterError(code: "INVALID_ARGS", message: "Missing arguments", details: nil))
      return
    }

    guard let texture = textures[textureId] else {
      result(FlutterError(code: "NOT_FOUND", message: "Texture not found", details: nil))
      return
    }

    if texture.render(editorPtr: editorPtr, pageIndex: pageIndex, width: width, height: height) {
      textureRegistry.textureFrameAvailable(textureId)
      result(true)
    } else {
      result(FlutterError(code: "RENDER_FAILED", message: "Render failed", details: nil))
    }
  }

  private func handleDispose(_ call: FlutterMethodCall, result: @escaping FlutterResult) {
    guard let args = call.arguments as? [String: Any],
          let textureId = args["textureId"] as? Int64 else {
      result(FlutterError(code: "INVALID_ARGS", message: "Missing textureId", details: nil))
      return
    }

    if let texture = textures.removeValue(forKey: textureId) {
      texture.dispose()
      textureRegistry.unregisterTexture(textureId)
    }

    result(nil)
  }
}

class EditorTexture: NSObject, FlutterTexture {
  private var frontBuffer: CVPixelBuffer?
  private var backBuffer: CVPixelBuffer?
  private(set) var currentWidth: Int
  private(set) var currentHeight: Int
  private(set) var currentStride: Int = 0
  private let bufferLock = NSLock()

  init(width: Int, height: Int) {
    self.currentWidth = width
    self.currentHeight = height
    super.init()
    createBuffer(width: width, height: height)
  }

  private func createBuffer(width: Int, height: Int) {
    frontBuffer = nil
    backBuffer = nil

    let attrs: [String: Any] = [
      kCVPixelBufferIOSurfacePropertiesKey as String: [:] as [String: Any],
      kCVPixelBufferMetalCompatibilityKey as String: true
    ]

    var front: CVPixelBuffer?
    var back: CVPixelBuffer?

    CVPixelBufferCreate(
      kCFAllocatorDefault,
      width,
      height,
      kCVPixelFormatType_32BGRA,
      attrs as CFDictionary,
      &front
    )

    CVPixelBufferCreate(
      kCFAllocatorDefault,
      width,
      height,
      kCVPixelFormatType_32BGRA,
      attrs as CFDictionary,
      &back
    )

    self.frontBuffer = front
    self.backBuffer = back
    self.currentWidth = width
    self.currentHeight = height
    if let back = back {
      self.currentStride = CVPixelBufferGetBytesPerRow(back)
    }
  }

  func render(editorPtr: Int64, pageIndex: Int, width: Int, height: Int) -> Bool {
    guard bufferLock.try() else { return false }
    defer { bufferLock.unlock() }

    if width != currentWidth || height != currentHeight {
      createBuffer(width: width, height: height)
    }

    guard let buffer = backBuffer else { return false }

    CVPixelBufferLockBaseAddress(buffer, [])
    defer { CVPixelBufferUnlockBaseAddress(buffer, []) }

    guard let ptr = CVPixelBufferGetBaseAddress(buffer) else {
      return false
    }

    let stride = CVPixelBufferGetBytesPerRow(buffer)
    currentStride = stride

    let editorHandle = UnsafeMutableRawPointer(bitPattern: Int(editorPtr))
    let result = editor_render_page_to(
      OpaquePointer(editorHandle),
      pageIndex,
      ptr.assumingMemoryBound(to: UInt8.self),
      stride,
      height,
      PIXEL_FORMAT_BGRA
    )

    swap(&frontBuffer, &backBuffer)

    return result == 0
  }

  func copyPixelBuffer() -> Unmanaged<CVPixelBuffer>? {
    guard let buffer = frontBuffer else { return nil }
    return Unmanaged.passRetained(buffer)
  }

  func dispose() {
    bufferLock.lock()
    defer { bufferLock.unlock() }
    frontBuffer = nil
    backBuffer = nil
  }
}
