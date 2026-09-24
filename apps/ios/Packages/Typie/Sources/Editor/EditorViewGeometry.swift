import CoreGraphics
internal import EditorFFI

struct EditorInsets: Equatable {
  var top: Double
  var left: Double
  var bottom: Double
  var right: Double
}

struct EditorBodyAreas: Equatable {
  var spacer: CGRect
  var padding: CGRect
  var fill: CGRect
}

struct EditorSurfaceDebug: Equatable {
  var size: CGSize
  var isEven: Bool
  var bottomMargin: Double
  var pending: [CGRect]
  var invalidated: [CGRect]
}

enum EditorViewGeometry {
  static let cropMarkerLength: Double = 32

  static func request(
    size: CGSize, offset: CGPoint, insets: EditorInsets, deviceScale: Double, timeMs: Double,
    debug: Bool, destination: Double?
  ) -> ViewportRequest {
    ViewportRequest(
      scrollX: offset.x + insets.left, scrollY: offset.y + insets.top,
      width: max(0, size.width - insets.left - insets.right),
      height: max(0, size.height - insets.top - insets.bottom), occlusionTop: 0,
      occlusionBottom: 0, deviceScale: deviceScale, headerHeight: 0, timeMs: timeMs,
      debug: debug, destination: destination.map { $0 + insets.top })
  }

  static func bodyAreas(_ geometry: FrameGeometry) -> EditorBodyAreas {
    let body = geometry.body
    let column = geometry.pages.max { $0.width < $1.width }
    let x = column?.x ?? 0
    let width = column?.width ?? geometry.contentWidth
    let fillTop = body.pagesBottom + body.bottomPadding
    return EditorBodyAreas(
      spacer: CGRect(
        x: x, y: body.pagesTop - body.topSpacer, width: width, height: body.topSpacer),
      padding: CGRect(x: x, y: body.pagesBottom, width: width, height: body.bottomPadding),
      fill: CGRect(
        x: x, y: fillTop, width: width, height: max(0, body.minimumBodyBottom - fillTop)))
  }

  static func surfaceDebug(page: Int, size: CGSize, geometry: FrameGeometry)
    -> EditorSurfaceDebug
  {
    let deviceScale = geometry.rasterScale / geometry.zoom
    let rects = { (tiles: [FrameTileRef]) in
      tiles.filter { Int($0.page) == page }.map {
        EditorSurfaceModel.layerFrame($0.bounds, deviceScale: deviceScale)
      }
    }
    var bottomMargin: Double = 0
    if case .paginated(_, let marginBottom, _, _) = geometry.layout {
      let pixels = (marginBottom * geometry.zoom * deviceScale).rounded()
      bottomMargin = min(max(0, pixels / deviceScale), Double(size.height))
    }
    return EditorSurfaceDebug(
      size: size, isEven: page % 2 == 0, bottomMargin: bottomMargin,
      pending: rects(geometry.debug?.pending ?? []),
      invalidated: rects(geometry.debug?.invalidated ?? []))
  }

  static func cropMarkers(layout: FrameLayout, zoom: Double, size: CGSize) -> [[CGPoint]] {
    guard case .paginated(let marginTop, let marginBottom, let marginLeft, let marginRight) = layout
    else { return [] }
    let length = cropMarkerLength * zoom
    let left = marginLeft * zoom
    let top = marginTop * zoom
    let right = Double(size.width) - marginRight * zoom
    let bottom = Double(size.height) - marginBottom * zoom
    return [
      (left, top, -length, -length), (right, top, length, -length),
      (left, bottom, -length, length), (right, bottom, length, length),
    ].map { x, y, outward, vertical in
      [CGPoint(x: x, y: y + vertical), CGPoint(x: x, y: y), CGPoint(x: x + outward, y: y)]
    }
  }
}
