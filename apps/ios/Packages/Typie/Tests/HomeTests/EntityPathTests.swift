import Testing

@testable import Home

@Suite struct EntityPathTests {
  @Test func rootOnlyWhenNoAncestors() {
    #expect(EntityPath.candidates([]) == ["작업실"])
  }

  @Test func candidatesDropFromTheHead() {
    #expect(
      EntityPath.candidates(["2026", "워크숍", "2차"]) == [
        "작업실 › 2026 › 워크숍 › 2차", "… › 2026 › 워크숍 › 2차", "… › 워크숍 › 2차", "… › 2차",
      ])
  }

  @Test func choosesFirstFitting() {
    let chosen = EntityPath.choose(["2026", "워크숍", "2차"]) { $0.count <= 12 }
    #expect(chosen == "… › 워크숍 › 2차")
  }

  @Test func fallsBackToLastCandidate() {
    #expect(EntityPath.choose(["아주아주아주긴폴더이름"]) { _ in false } == "… › 아주아주아주긴폴더이름")
    #expect(EntityPath.choose([]) { _ in false } == "작업실")
  }
}
