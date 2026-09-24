import CoreGraphics
import EditorFFI
import Testing

@testable import Editor

@Suite struct EditorViewGeometryTests {
  private let bars = EditorInsets(top: 116, left: 0, bottom: 34, right: 0)
  private let screen = CGSize(width: 402, height: 874)

  @Test func theViewportIsTheAreaBetweenTheBars() {
    let request = EditorViewGeometry.request(
      size: screen, offset: CGPoint(x: 0, y: -116), insets: bars, deviceScale: 3, timeMs: 5,
      debug: true, destination: nil)

    #expect(
      request
        == ViewportRequest(
          scrollX: 0, scrollY: 0, width: 402, height: 724, occlusionTop: 0, occlusionBottom: 0,
          deviceScale: 3, headerHeight: 0, timeMs: 5, debug: true, destination: nil))
  }

  @Test func scrollOffsetsBecomeContentCoordinates() {
    let request = EditorViewGeometry.request(
      size: screen, offset: CGPoint(x: 0, y: 500), insets: bars, deviceScale: 3, timeMs: 0,
      debug: false, destination: nil)

    #expect(request.scrollY == 616)
  }

  @Test func bouncingPastTheTopReportsANegativeScroll() {
    let request = EditorViewGeometry.request(
      size: screen, offset: CGPoint(x: 0, y: -200), insets: bars, deviceScale: 3, timeMs: 0,
      debug: false, destination: nil)

    #expect(request.scrollY == -84)
  }

  @Test func destinationsMoveLikeTheScrollOffset() {
    let fling = EditorViewGeometry.request(
      size: screen, offset: CGPoint(x: 0, y: 500), insets: bars, deviceScale: 3, timeMs: 0,
      debug: false, destination: 2000)
    let top = EditorViewGeometry.request(
      size: screen, offset: CGPoint(x: 0, y: 500), insets: bars, deviceScale: 3, timeMs: 0,
      debug: false, destination: -116)

    #expect(fling.scrollY == 616)
    #expect(fling.destination == 2116)
    #expect(top.destination == 0)
  }

  @Test func withoutADestinationTheRequestHasNone() {
    let request = EditorViewGeometry.request(
      size: screen, offset: CGPoint(x: 0, y: 500), insets: bars, deviceScale: 3, timeMs: 0,
      debug: false, destination: nil)

    #expect(request.destination == nil)
  }

  @Test func sideInsetsNarrowTheViewport() {
    let request = EditorViewGeometry.request(
      size: CGSize(width: 874, height: 402), offset: CGPoint(x: -59, y: -44),
      insets: EditorInsets(top: 44, left: 59, bottom: 21, right: 59), deviceScale: 3, timeMs: 0,
      debug: false, destination: nil)

    #expect(request.scrollX == 0)
    #expect(request.width == 756)
    #expect(request.height == 337)
  }

  @Test func insetsLargerThanTheViewLeaveAnEmptyViewport() {
    let request = EditorViewGeometry.request(
      size: CGSize(width: 100, height: 100), offset: .zero,
      insets: EditorInsets(top: 80, left: 60, bottom: 40, right: 60), deviceScale: 3, timeMs: 0,
      debug: false, destination: nil)

    #expect(request.width == 0)
    #expect(request.height == 0)
  }

  @Test func bodyAreasSpanTheContentWidth() {
    let areas = EditorViewGeometry.bodyAreas(
      geometry(
        body: FrameBody(
          pagesTop: 40, topSpacer: 40, pagesBottom: 3000, bottomPadding: 300,
          minimumBodyBottom: 3500),
        pages: [FramePage(index: 0, x: 0, y: 40, width: 390, height: 2960)]))

    #expect(areas.spacer == CGRect(x: 0, y: 0, width: 390, height: 40))
    #expect(areas.padding == CGRect(x: 0, y: 3000, width: 390, height: 300))
    #expect(areas.fill == CGRect(x: 0, y: 3300, width: 390, height: 200))
  }

  @Test func bodyAreasFollowThePageColumnWhenTheViewportIsWider() {
    let areas = EditorViewGeometry.bodyAreas(
      geometry(
        layout: paginated, deviceScale: 2, contentWidth: 1024,
        body: FrameBody(
          pagesTop: 0, topSpacer: 0, pagesBottom: 842, bottomPadding: 0,
          minimumBodyBottom: 1300),
        pages: [FramePage(index: 0, x: 214.5, y: 0, width: 595, height: 842)]))

    #expect(areas.spacer == CGRect(x: 214.5, y: 0, width: 595, height: 0))
    #expect(areas.padding == CGRect(x: 214.5, y: 842, width: 595, height: 0))
    #expect(areas.fill == CGRect(x: 214.5, y: 842, width: 595, height: 458))
  }

  @Test func bodyAreasFollowTheWidestPage() {
    let areas = EditorViewGeometry.bodyAreas(
      geometry(
        deviceScale: 2, contentWidth: 1024,
        body: FrameBody(
          pagesTop: 40, topSpacer: 40, pagesBottom: 1724, bottomPadding: 300,
          minimumBodyBottom: 1300),
        pages: [
          FramePage(index: 0, x: 312, y: 40, width: 400, height: 842),
          FramePage(index: 1, x: 214.5, y: 882, width: 595, height: 842),
        ]))

    #expect(areas.spacer == CGRect(x: 214.5, y: 0, width: 595, height: 40))
    #expect(areas.padding == CGRect(x: 214.5, y: 1724, width: 595, height: 300))
  }

  @Test func bodyAreasFallBackToTheContentWidthWithoutPages() {
    let areas = EditorViewGeometry.bodyAreas(
      geometry(
        contentWidth: 1024,
        body: FrameBody(
          pagesTop: 40, topSpacer: 40, pagesBottom: 40, bottomPadding: 0,
          minimumBodyBottom: 1300)))

    #expect(areas.spacer == CGRect(x: 0, y: 0, width: 1024, height: 40))
    #expect(areas.fill == CGRect(x: 0, y: 40, width: 1024, height: 1260))
  }

  @Test func bodyFillIsEmptyWhenThePagesOutgrowTheViewport() {
    let areas = EditorViewGeometry.bodyAreas(
      geometry(
        body: FrameBody(
          pagesTop: 0, topSpacer: 0, pagesBottom: 9000, bottomPadding: 34,
          minimumBodyBottom: 690)))

    #expect(areas.fill.height == 0)
    #expect(areas.spacer.height == 0)
  }

  @Test func surfaceDebugPlacesThisPagesTilesInPagePoints() {
    let debug = FrameDebug(
      pending: [
        FrameTileRef(page: 0, bounds: FramePxRect(x0: 0, y0: 0, x1: 512, y1: 512)),
        FrameTileRef(page: 1, bounds: FramePxRect(x0: 512, y0: 0, x1: 1024, y1: 512)),
      ],
      invalidated: [
        FrameTileRef(page: 0, bounds: FramePxRect(x0: 512, y0: 512, x1: 1024, y1: 600)),
        FrameTileRef(page: 1, bounds: FramePxRect(x0: 0, y0: 512, x1: 512, y1: 600)),
      ]
    )
    let surface = EditorViewGeometry.surfaceDebug(
      page: 1, size: CGSize(width: 397, height: 561.5),
      geometry: geometry(layout: paginated, zoom: 0.5, debug: debug))

    #expect(
      surface
        == EditorSurfaceDebug(
          size: CGSize(width: 397, height: 561.5), isEven: false, bottomMargin: 47,
          pending: [CGRect(x: 512.0 / 3, y: 0, width: 512.0 / 3, height: 512.0 / 3)],
          invalidated: [CGRect(x: 0, y: 512.0 / 3, width: 512.0 / 3, height: 88.0 / 3)]))
  }

  @Test func continuousPagesHaveNoBottomMarginTint() {
    let surface = EditorViewGeometry.surfaceDebug(
      page: 0, size: CGSize(width: 390, height: 1000), geometry: geometry())

    #expect(surface.isEven)
    #expect(surface.bottomMargin == 0)
    #expect(surface.pending.isEmpty)
  }

  @Test func bottomMarginTintSnapsToDevicePixels() {
    let fractional = EditorViewGeometry.surfaceDebug(
      page: 0, size: CGSize(width: 223.25, height: 315.75),
      geometry: geometry(layout: paginated, zoom: 0.375))
    let tie = EditorViewGeometry.surfaceDebug(
      page: 0, size: CGSize(width: 400, height: 500),
      geometry: geometry(
        layout: .paginated(marginTop: 0, marginBottom: 1.25, marginLeft: 0, marginRight: 0),
        deviceScale: 2))

    #expect(fractional.bottomMargin == 106.0 / 3)
    #expect(tie.bottomMargin == 1.5)
  }

  @Test func bottomMarginTintStopsAtThePageHeight() {
    let surface = EditorViewGeometry.surfaceDebug(
      page: 0, size: CGSize(width: 397, height: 400),
      geometry: geometry(
        layout: .paginated(marginTop: 0, marginBottom: 900, marginLeft: 0, marginRight: 0),
        zoom: 0.5))

    #expect(surface.bottomMargin == 400)
  }

  @Test func cropMarkersLeaveTheMarginCornersOutward() {
    let markers = EditorViewGeometry.cropMarkers(
      layout: paginated, zoom: 0.5, size: CGSize(width: 397, height: 561.5))

    #expect(
      markers == [
        [CGPoint(x: 47, y: 31), CGPoint(x: 47, y: 47), CGPoint(x: 31, y: 47)],
        [CGPoint(x: 350, y: 31), CGPoint(x: 350, y: 47), CGPoint(x: 366, y: 47)],
        [CGPoint(x: 47, y: 530.5), CGPoint(x: 47, y: 514.5), CGPoint(x: 31, y: 514.5)],
        [CGPoint(x: 350, y: 530.5), CGPoint(x: 350, y: 514.5), CGPoint(x: 366, y: 514.5)],
      ])
  }

  @Test func continuousPagesHaveNoCropMarkers() {
    #expect(
      EditorViewGeometry.cropMarkers(
        layout: .continuous, zoom: 1, size: CGSize(width: 390, height: 1000)
      ).isEmpty)
  }

  private let paginated = FrameLayout.paginated(
    marginTop: 94, marginBottom: 94, marginLeft: 94, marginRight: 94)

  private func geometry(
    layout: FrameLayout = .continuous, zoom: Double = 1, deviceScale: Double = 3,
    contentWidth: Double = 390,
    body: FrameBody = FrameBody(
      pagesTop: 40, topSpacer: 40, pagesBottom: 1040, bottomPadding: 0, minimumBodyBottom: 844),
    pages: [FramePage] = [], debug: FrameDebug? = nil
  ) -> FrameGeometry {
    FrameGeometry(
      id: 1, revision: Revision(value: 1), tick: nil, layout: layout, zoom: zoom,
      rasterScale: zoom * deviceScale, contentWidth: contentWidth, contentHeight: 5000,
      pageCount: 2, body: body, pages: pages, tiles: [], position: nil, debug: debug,
      needsNextFrame: false, fillRemaining: false)
  }
}
