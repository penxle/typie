import Flutter
import UIKit

class EditorTexturePlugin: NSObject, FlutterPlugin {
  private let textureRegistry: FlutterTextureRegistry
  private var textures: [Int64: EditorTexture] = [:]
  private static let maxTextures = 10

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
          let items = args["items"] as? [[String: Any]] else {
      result(FlutterError(code: "INVALID_ARGS", message: "Missing items", details: nil))
      return
    }

    var results: [Bool] = []

    for item in items {
      guard let textureId = item["textureId"] as? Int64,
            let editorPtr = item["editorPtr"] as? Int64,
            let pageIndex = item["pageIndex"] as? Int,
            let width = item["width"] as? Int,
            let height = item["height"] as? Int else {
        results.append(false)
        continue
      }

      guard let texture = textures[textureId] else {
        results.append(false)
        continue
      }

      let didRender = texture.render(editorPtr: editorPtr, pageIndex: pageIndex, width: width, height: height)
      if didRender {
        textureRegistry.textureFrameAvailable(textureId)
      }
      results.append(didRender)
    }

    result(results)
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

    let editorHandle = UnsafeMutableRawPointer(bitPattern: Int(editorPtr))
    let result = editor_render_page_to(
      OpaquePointer(editorHandle),
      pageIndex,
      ptr.assumingMemoryBound(to: UInt8.self),
      stride,
      width,
      height,
      PIXEL_FORMAT_BGRA
    )

    if result == 0 {
      swap(&frontBuffer, &backBuffer)
      return true
    }

    return false
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
