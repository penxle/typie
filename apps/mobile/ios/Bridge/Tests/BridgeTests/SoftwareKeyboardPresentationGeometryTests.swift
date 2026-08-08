import CoreGraphics
import XCTest
@testable import Bridge

final class SoftwareKeyboardPresentationGeometryTests: XCTestCase {
  func testHostAndInsetFramesFollowTheSameProgressAcrossDifferentTravelDistances() {
    let geometry = SoftwareKeyboardPresentationGeometry(
      shownHostBounds: CGRect(x: 0, y: 7, width: 393, height: 351),
      shownFrame: CGRect(x: 0, y: 535, width: 393, height: 317),
      hostTravel: 351,
      frameTravel: 317
    )

    let hostBounds = geometry.hostBounds(at: 0.5)
    let keyboardFrame = geometry.keyboardFrame(at: 0.5)

    XCTAssertEqual(hostBounds.minY, -168.5, accuracy: 0.001)
    XCTAssertEqual(keyboardFrame.minY, 693.5, accuracy: 0.001)
    XCTAssertEqual(geometry.progress(for: hostBounds), 0.5, accuracy: 0.001)
  }
}
