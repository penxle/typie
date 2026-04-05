import QuartzCore

@objc public class MetalSurfaceBridge: NSObject {
    @objc public static func pointerOf(_ layer: CAMetalLayer) -> Int64 {
        Int64(Int(bitPattern: Unmanaged.passUnretained(layer).toOpaque()))
    }
}
