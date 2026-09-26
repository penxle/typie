import CoreGraphics
internal import EditorFFI

struct SurfacePage: Equatable {
  var index: Int
  var frame: CGRect
}

struct SurfaceTile {
  var key: EditorTileKey
  var image: CGImage
  var frame: CGRect
  var contentsRect: CGRect
}

struct SurfaceChanges {
  var contentSize: CGSize
  var addedPages: [SurfacePage] = []
  var movedPages: [SurfacePage] = []
  var removedPages: [Int] = []
  var droppedTiles: [EditorTileKey] = []
  var setTiles: [SurfaceTile] = []
}

struct EditorSurfaceModel {
  private(set) var pages: [Int: CGRect] = [:]
  private(set) var tiles: [EditorTileKey: UInt64] = [:]
  private(set) var contentSize: CGSize = .zero

  mutating func apply(_ update: EditorFrameUpdate) -> SurfaceChanges {
    let geometry = update.geometry
    let deviceScale = geometry.rasterScale / geometry.zoom
    contentSize = CGSize(width: geometry.contentWidth, height: geometry.contentHeight)
    var changes = SurfaceChanges(contentSize: contentSize)

    for change in update.tiles {
      guard case .drop(let key) = change, tiles.removeValue(forKey: key) != nil else { continue }
      changes.droppedTiles.append(key)
    }

    var next: [Int: CGRect] = [:]
    for page in geometry.pages {
      next[Int(page.index)] = CGRect(x: page.x, y: page.y, width: page.width, height: page.height)
    }
    for index in pages.keys.sorted() where next[index] == nil {
      changes.removedPages.append(index)
      for key in tiles.keys.filter({ $0.page == index }).sorted(by: Self.order) {
        tiles[key] = nil
        changes.droppedTiles.append(key)
      }
    }
    for index in next.keys.sorted() {
      let frame = next[index]!
      switch pages[index] {
      case nil: changes.addedPages.append(SurfacePage(index: index, frame: frame))
      case let old? where old != frame:
        changes.movedPages.append(SurfacePage(index: index, frame: frame))
      default: break
      }
    }
    pages = next

    for change in update.tiles {
      guard case .set(let key, let version, let image) = change, pages[key.page] != nil else {
        continue
      }
      tiles[key] = version
      changes.setTiles.append(
        SurfaceTile(
          key: key, image: image, frame: Self.layerFrame(key.bounds, deviceScale: deviceScale),
          contentsRect: Self.contentsRect(key.bounds)))
    }
    return changes
  }

  static func layerFrame(_ bounds: FramePxRect, deviceScale: Double) -> CGRect {
    CGRect(
      x: Double(bounds.x0) / deviceScale, y: Double(bounds.y0) / deviceScale,
      width: Double(bounds.x1 - bounds.x0) / deviceScale,
      height: Double(bounds.y1 - bounds.y0) / deviceScale)
  }

  static func contentsRect(_ bounds: FramePxRect) -> CGRect {
    let width = Double(bounds.x1 - bounds.x0)
    let height = Double(bounds.y1 - bounds.y0)
    return CGRect(
      x: 1 / (width + 2), y: 1 / (height + 2), width: width / (width + 2),
      height: height / (height + 2))
  }

  private static func order(_ lhs: EditorTileKey, _ rhs: EditorTileKey) -> Bool {
    (lhs.bounds.y0, lhs.bounds.x0) < (rhs.bounds.y0, rhs.bounds.x0)
  }
}
