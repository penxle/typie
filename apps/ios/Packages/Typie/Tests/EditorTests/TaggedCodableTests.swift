import EditorFFI
import Foundation
import Testing

@Suite struct TaggedCodableTests {
  private func json<Value: Encodable>(_ value: Value) throws -> String {
    let encoder = JSONEncoder()
    encoder.outputFormatting = .sortedKeys
    return String(decoding: try encoder.encode(value), as: UTF8.self)
  }

  @Test func adjacentlyTaggedAttributesMatchTheEngineWireShape() throws {
    let attribute = NodeAttr.image(attr: .proportion(65))
    let expected = #"{"attr":{"type":"proportion","value":65},"type":"image"}"#
    #expect(try json(attribute) == expected)
    #expect(try JSONDecoder().decode(NodeAttr.self, from: Data(expected.utf8)) == attribute)
  }

  @Test func adjacentlyTaggedOptionalPayloadKeepsTheContentKey() throws {
    let attribute = NodeAttr.image(attr: .id("asset-1"))
    let expected = #"{"attr":{"type":"id","value":"asset-1"},"type":"image"}"#
    #expect(try json(attribute) == expected)
    #expect(try JSONDecoder().decode(NodeAttr.self, from: Data(expected.utf8)) == attribute)
  }

  @Test func externallyTaggedStructVariantUsesTheVariantKey() throws {
    let effect = Effect.loadFont(family: "SUIT", weight: 400, codepoints: [65])
    let expected = #"{"load_font":{"codepoints":[65],"family":"SUIT","weight":400}}"#
    #expect(try json(effect) == expected)
    #expect(try JSONDecoder().decode(Effect.self, from: Data(expected.utf8)) == effect)
  }

  @Test func unknownVariantsFailToDecode() {
    #expect(throws: DecodingError.self) {
      try JSONDecoder().decode(Message.self, from: Data(#"{"type":"unknown_kind"}"#.utf8))
    }
  }
}
