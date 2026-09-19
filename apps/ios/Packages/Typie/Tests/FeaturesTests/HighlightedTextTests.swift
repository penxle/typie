import Testing

@testable import Features

@Suite struct HighlightedTextTests {
  private func parts(_ markup: String) -> [(String, Bool)] {
    HighlightedText(markup: markup).segments.map { ($0.text, $0.isHighlighted) }
  }

  @Test func plainTextHasOneSegment() {
    let text = HighlightedText(markup: "abc")
    #expect(text.segments.count == 1)
    #expect(text.segments[0].text == "abc")
    #expect(text.segments[0].isHighlighted == false)
    #expect(text.plain == "abc")
  }

  @Test func splitsAroundEm() {
    let result = parts("a <em>b</em> c <em>d</em>")
    #expect(result.map(\.0) == ["a ", "b", " c ", "d"])
    #expect(result.map(\.1) == [false, true, false, true])
  }

  @Test func unclosedEmStaysPlain() {
    let result = parts("a <em>b")
    #expect(result.map(\.0) == ["a ", "<em>b"])
    #expect(result.map(\.1) == [false, false])
  }

  @Test func decodesEntities() {
    #expect(HighlightedText(markup: "a &amp; b &lt;c&gt;&nbsp;d").plain == "a & b <c>\u{00A0}d")
    #expect(HighlightedText(markup: "&amp;lt;").plain == "&lt;")
  }

  @Test func plainInitializerDoesNotParse() {
    #expect(HighlightedText(plain: "<em>x</em>").plain == "<em>x</em>")
  }
}
