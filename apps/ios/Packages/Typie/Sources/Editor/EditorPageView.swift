#if canImport(UIKit)

  import Design
  internal import EditorFFI
  import UIKit

  final class EditorPageView: UIView {
    private static let cropMarkerOpacity: CGFloat = 0.15
    private static let boundaryThickness: CGFloat = 2

    private let tileHost = CALayer()
    private let cropMarkers = CAShapeLayer()
    private let debugHost = CALayer()
    private var tiles: [FramePxRect: CALayer] = [:]
    private var layout = FrameLayout.continuous
    private var zoom: Double = 1
    private var showsCropMarkers = false
    private var surfaceDebug: EditorSurfaceDebug?

    override init(frame: CGRect) {
      super.init(frame: frame)
      isUserInteractionEnabled = false
      cropMarkers.fillColor = nil
      layer.addSublayer(tileHost)
      layer.addSublayer(cropMarkers)
      layer.addSublayer(debugHost)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func layoutSubviews() {
      super.layoutSubviews()
      CATransaction.begin()
      CATransaction.setDisableActions(true)
      for host in [tileHost, cropMarkers, debugHost] {
        host.frame = bounds
      }
      updateCropMarkers()
      CATransaction.commit()
    }

    func configure(layout: FrameLayout, zoom: Double, isEditable: Bool, traits: UITraitCollection) {
      self.layout = layout
      self.zoom = zoom
      switch layout {
      case .paginated:
        backgroundColor = .theme(\.surfaceDefault)
        layer.borderWidth = 1
        showsCropMarkers = isEditable
      case .continuous:
        backgroundColor = nil
        layer.borderWidth = 0
        showsCropMarkers = false
      }
      resolveColors(traits)
      updateCropMarkers()
    }

    func resolveColors(_ traits: UITraitCollection) {
      layer.borderColor = UIColor.theme(\.borderHairline).resolvedColor(with: traits).cgColor
      cropMarkers.strokeColor =
        UIColor.theme(\.textDefault).resolvedColor(with: traits)
        .withAlphaComponent(Self.cropMarkerOpacity).cgColor
    }

    func setTile(_ tile: SurfaceTile) {
      let tileLayer = tiles[tile.key.bounds] ?? CALayer()
      if tileLayer.superlayer == nil {
        tileHost.addSublayer(tileLayer)
        tiles[tile.key.bounds] = tileLayer
      }
      tileLayer.contents = tile.image
      tileLayer.contentsRect = tile.contentsRect
      tileLayer.frame = tile.frame
    }

    func dropTile(_ bounds: FramePxRect) {
      tiles.removeValue(forKey: bounds)?.removeFromSuperlayer()
    }

    func showSurfaceDebug(_ debug: EditorSurfaceDebug?) {
      guard debug != surfaceDebug else { return }
      surfaceDebug = debug
      debugHost.sublayers = nil
      guard let debug else { return }
      addDebugRect(bounds, debug.isEven ? EditorDebugColors.evenPage : EditorDebugColors.oddPage)
      if debug.bottomMargin > 0 {
        addDebugRect(
          CGRect(
            x: 0, y: bounds.height - debug.bottomMargin, width: bounds.width,
            height: debug.bottomMargin),
          EditorDebugColors.bottomMargin)
      }
      for rect in debug.pending {
        addDebugRect(rect, EditorDebugColors.pendingTile)
      }
      for rect in debug.invalidated {
        addDebugRect(rect, EditorDebugColors.invalidatedTile)
      }
      addDebugRect(
        CGRect(x: 0, y: 0, width: bounds.width, height: Self.boundaryThickness),
        EditorDebugColors.pageBoundary)
      addDebugRect(
        CGRect(
          x: 0, y: bounds.height - Self.boundaryThickness, width: bounds.width,
          height: Self.boundaryThickness),
        EditorDebugColors.pageBoundary)
    }

    private func addDebugRect(_ rect: CGRect, _ color: UIColor) {
      let tint = CALayer()
      tint.frame = rect
      tint.backgroundColor = color.cgColor
      debugHost.addSublayer(tint)
    }

    private func updateCropMarkers() {
      let path = CGMutablePath()
      if showsCropMarkers {
        for points in EditorViewGeometry.cropMarkers(layout: layout, zoom: zoom, size: bounds.size)
        {
          path.addLines(between: points)
        }
      }
      cropMarkers.path = path.isEmpty ? nil : path
      cropMarkers.lineWidth = zoom
    }
  }

  @MainActor
  enum EditorDebugColors {
    static let viewportGuide = UIColor(argb: 0xFF00_C853).withAlphaComponent(0.9)
    static let topSpacer = UIColor(argb: 0x22FF_5ACD)
    static let bottomPadding = UIColor(argb: 0x22FF_8A00)
    static let extensionFill = UIColor(argb: 0x2200_B8D4)
    static let evenPage = UIColor(argb: 0x2200_96FF)
    static let oddPage = UIColor(argb: 0x2234_C759)
    static let bottomMargin = UIColor(argb: 0x22FF_D600)
    static let pendingTile = UIColor(argb: 0x9900_66FF)
    static let invalidatedTile = UIColor(argb: 0x9930_D158)
    static let pageBoundary = UIColor(argb: 0xE6FF_3B30)
  }

  extension UIColor {
    convenience init(argb: UInt32) {
      self.init(
        red: CGFloat((argb >> 16) & 0xFF) / 255, green: CGFloat((argb >> 8) & 0xFF) / 255,
        blue: CGFloat(argb & 0xFF) / 255, alpha: CGFloat(argb >> 24) / 255)
    }
  }

#endif
