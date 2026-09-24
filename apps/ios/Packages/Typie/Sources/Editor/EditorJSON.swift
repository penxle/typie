import Foundation

enum EditorJSON {
  static func encode<Value: Encodable>(_ value: Value) throws -> String {
    String(decoding: try JSONEncoder().encode(value), as: UTF8.self)
  }

  static func decode<Value: Decodable>(_ type: Value.Type, from json: String) throws -> Value {
    try JSONDecoder().decode(type, from: Data(json.utf8))
  }
}
