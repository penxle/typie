import Design
import SwiftUI
import Testing

@testable import Features

@Suite struct EntityIconTests {
  private let colors = TColors.light

  @Test func mapsKnownNameAndColor() {
    let spec = EntityIconSpec(kind: .document, name: "rocket", color: "blue")
    let appearance = EntityIcon.appearance(spec, colors: colors)
    #expect(appearance.icon == LucideIcon.rocket)
    #expect(appearance.tint == colors.paletteBlue)
  }

  @Test func trimsName() {
    let spec = EntityIconSpec(kind: .document, name: " star ", color: " red ")
    let appearance = EntityIcon.appearance(spec, colors: colors)
    #expect(appearance.icon == LucideIcon.star)
    #expect(appearance.tint == colors.paletteRed)
  }

  @Test func fallsBackByKind() {
    let folder = EntityIcon.appearance(
      EntityIconSpec(kind: .folder, name: "", color: ""), colors: colors)
    #expect(folder.icon == LucideIcon.folder)
    #expect(folder.tint == colors.textMuted)
    let document = EntityIcon.appearance(
      EntityIconSpec(kind: .document, name: "unknown-name", color: "teal"), colors: colors)
    #expect(document.icon == LucideIcon.file)
    #expect(document.tint == colors.textMuted)
  }

  @Test func mapsRenamedLucideNames() {
    #expect(EntityIcon.names["home"] == LucideIcon.house)
    #expect(EntityIcon.names["fingerprint"] == LucideIcon.fingerprintPattern)
    #expect(EntityIcon.names["bar-chart-2"] == LucideIcon.barChartBig)
    #expect(EntityIcon.names["package"] == LucideIcon.package2)
    #expect(EntityIcon.names.count == 129)
  }

  @Test func mapsAllSevenColors() {
    let expected: [(String, Color)] = [
      ("gray", colors.paletteGray), ("red", colors.paletteRed), ("orange", colors.paletteOrange),
      ("yellow", colors.paletteYellow), ("green", colors.paletteGreen),
      ("blue", colors.paletteBlue),
      ("purple", colors.palettePurple),
    ]
    for (name, color) in expected {
      #expect(EntityIcon.tint(name, colors: colors) == color)
    }
  }
}
