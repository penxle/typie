import CoreGraphics
import EditorFFI
import Testing

@testable import Editor

@Suite struct EditorSurfaceModelTests {
  @Test func firstFrameAddsItsPagesAndPlacesTilesInPagePoints() {
    var model = EditorSurfaceModel()
    let key = tileKey(1, 512, 1024, 1024, 1536)
    let changes = model.apply(
      testUpdate(
        pages: [testPage(0, y: 40), testPage(1, y: 1040)],
        tiles: [.set(key, version: 1, image: testImage())]))

    #expect(changes.contentSize == CGSize(width: 390, height: 5000))
    #expect(
      changes.addedPages == [
        SurfacePage(index: 0, frame: CGRect(x: 0, y: 40, width: 390, height: 1000)),
        SurfacePage(index: 1, frame: CGRect(x: 0, y: 1040, width: 390, height: 1000)),
      ])
    #expect(changes.movedPages.isEmpty)
    #expect(changes.removedPages.isEmpty)
    #expect(changes.setTiles.map(\.key) == [key])
    #expect(
      changes.setTiles[0].frame
        == CGRect(x: 512.0 / 3, y: 1024.0 / 3, width: 512.0 / 3, height: 512.0 / 3))
    #expect(
      changes.setTiles[0].contentsRect
        == CGRect(x: 1.0 / 514, y: 1.0 / 514, width: 512.0 / 514, height: 512.0 / 514))
    #expect(model.tiles == [key: 1])
    #expect(model.pages.keys.sorted() == [0, 1])
  }

  @Test func layerFramesDivideByTheDeviceScaleWhateverTheZoom() {
    var model = EditorSurfaceModel()
    let key = tileKey(0, 0, 0, 512, 300)
    let changes = model.apply(
      testUpdate(
        pages: [testPage(0, y: 0)], tiles: [.set(key, version: 1, image: testImage())],
        zoom: 0.5, deviceScale: 2))

    #expect(changes.setTiles[0].frame == CGRect(x: 0, y: 0, width: 256, height: 150))
  }

  @Test func edgeTilesCropTheirOwnGutter() {
    #expect(
      EditorSurfaceModel.contentsRect(FramePxRect(x0: 1024, y0: 3072, x1: 1124, y1: 3109))
        == CGRect(x: 1.0 / 102, y: 1.0 / 39, width: 100.0 / 102, height: 37.0 / 39))
  }

  @Test func aNewVersionReplacesTheTile() {
    var model = EditorSurfaceModel()
    let key = tileKey(0, 0, 0, 512, 512)
    _ = model.apply(
      testUpdate(pages: [testPage(0, y: 0)], tiles: [.set(key, version: 1, image: testImage())]))
    let changes = model.apply(
      testUpdate(pages: [testPage(0, y: 0)], tiles: [.set(key, version: 7, image: testImage())]))

    #expect(changes.addedPages.isEmpty)
    #expect(changes.droppedTiles.isEmpty)
    #expect(changes.setTiles.map(\.key) == [key])
    #expect(model.tiles == [key: 7])
  }

  @Test func droppedTilesLeaveTheSurface() {
    var model = EditorSurfaceModel()
    let kept = tileKey(0, 0, 0, 512, 512)
    let dropped = tileKey(0, 0, 512, 512, 1024)
    _ = model.apply(
      testUpdate(
        pages: [testPage(0, y: 0)],
        tiles: [
          .set(kept, version: 1, image: testImage()),
          .set(dropped, version: 2, image: testImage()),
        ]))
    let changes = model.apply(testUpdate(pages: [testPage(0, y: 0)], tiles: [.drop(dropped)]))

    #expect(changes.droppedTiles == [dropped])
    #expect(changes.setTiles.isEmpty)
    #expect(model.tiles == [kept: 1])
  }

  @Test func dropsForTilesTheSurfaceNeverHadAreIgnored() {
    var model = EditorSurfaceModel()
    let changes = model.apply(
      testUpdate(pages: [testPage(0, y: 0)], tiles: [.drop(tileKey(0, 0, 0, 512, 512))]))

    #expect(changes.droppedTiles.isEmpty)
  }

  @Test func pagesLeavingTheSetTakeTheirTilesWithThem() {
    var model = EditorSurfaceModel()
    let first = tileKey(0, 0, 512, 512, 1024)
    let second = tileKey(0, 512, 0, 1024, 512)
    let other = tileKey(1, 0, 0, 512, 512)
    _ = model.apply(
      testUpdate(
        pages: [testPage(0, y: 0), testPage(1, y: 1000)],
        tiles: [
          .set(first, version: 1, image: testImage()),
          .set(second, version: 2, image: testImage()),
          .set(other, version: 3, image: testImage()),
        ]))
    let changes = model.apply(testUpdate(pages: [testPage(1, y: 1000)]))

    #expect(changes.removedPages == [0])
    #expect(changes.droppedTiles == [second, first])
    #expect(model.tiles == [other: 3])
    #expect(model.pages.keys.sorted() == [1])
  }

  @Test func pagesLeavingWithTheirDropsReleaseEachTileOnce() {
    var model = EditorSurfaceModel()
    let first = tileKey(0, 0, 512, 512, 1024)
    let second = tileKey(0, 512, 0, 1024, 512)
    let other = tileKey(1, 0, 0, 512, 512)
    _ = model.apply(
      testUpdate(
        pages: [testPage(0, y: 0), testPage(1, y: 1000)],
        tiles: [
          .set(first, version: 1, image: testImage()),
          .set(second, version: 2, image: testImage()),
          .set(other, version: 3, image: testImage()),
        ]))
    let changes = model.apply(
      testUpdate(pages: [testPage(1, y: 1000)], tiles: [.drop(first), .drop(second)]))

    #expect(changes.removedPages == [0])
    #expect(changes.droppedTiles == [first, second])
    #expect(model.tiles == [other: 3])
  }

  @Test func pagesThatChangePlaceAreReportedAsMoved() {
    var model = EditorSurfaceModel()
    let key = tileKey(1, 0, 0, 512, 512)
    _ = model.apply(
      testUpdate(
        pages: [testPage(0, y: 40), testPage(1, y: 1040)],
        tiles: [.set(key, version: 1, image: testImage())]))
    let changes = model.apply(
      testUpdate(
        pages: [testPage(0, y: 40), testPage(1, y: 1060, height: 900), testPage(2, y: 1960)],
        contentHeight: 6000))

    #expect(changes.contentSize == CGSize(width: 390, height: 6000))
    #expect(
      changes.movedPages == [
        SurfacePage(index: 1, frame: CGRect(x: 0, y: 1060, width: 390, height: 900))
      ])
    #expect(
      changes.addedPages == [
        SurfacePage(index: 2, frame: CGRect(x: 0, y: 1960, width: 390, height: 1000))
      ])
    #expect(changes.droppedTiles.isEmpty)
    #expect(changes.setTiles.isEmpty)
    #expect(model.tiles == [key: 1])
  }

  @Test func tilesForPagesOutsideTheSetAreNotShown() {
    var model = EditorSurfaceModel()
    let changes = model.apply(
      testUpdate(
        pages: [testPage(0, y: 0)],
        tiles: [.set(tileKey(3, 0, 0, 512, 512), version: 1, image: testImage())]))

    #expect(changes.setTiles.isEmpty)
    #expect(model.tiles.isEmpty)
  }
}
