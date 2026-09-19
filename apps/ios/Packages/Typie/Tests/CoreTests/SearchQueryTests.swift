import GraphQL
import Testing

@testable import Core

@Suite struct SearchQueryTests {
  @Test func operationIsNamedByConvention() {
    #expect(SearchScreen_Search_Query.operationName == "SearchScreen_Search_Query")
    let query = SearchScreen_Search_Query(siteId: "site-1", query: "hello")
    #expect(query.siteId == "site-1")
    #expect(query.query == "hello")
  }

  @Test func dateTimeIsAString() {
    let raw: DateTime = "2027-01-02T03:04:05.678Z"
    #expect(parseDateTime(raw) != nil)
  }
}
