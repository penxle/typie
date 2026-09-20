@_spi(Internal) @_spi(Execution) import ApolloAPI

public struct JSON: CustomScalarType, Hashable, Sendable {
  public let value: JSONValue

  public init(_ value: JSONValue) {
    self.value = value
  }

  @_spi(Internal)
  public init(_jsonValue value: JSONValue) throws {
    self.value = value
  }

  @_spi(Internal)
  public var _jsonValue: JSONValue { value }

  public static func == (lhs: JSON, rhs: JSON) -> Bool {
    AnyHashable(lhs.value) == AnyHashable(rhs.value)
  }

  public func hash(into hasher: inout Hasher) {
    AnyHashable(value).hash(into: &hasher)
  }
}
