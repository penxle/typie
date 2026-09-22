import Observation

@MainActor @Observable
final class HeroTitleState {
  var titleVisible = false

  init() {}
}
