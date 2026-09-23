import Core
import FactoryKit
import GraphQL
import Observation

@MainActor @Observable
final class ProfileModel {
  struct Profile: Equatable {
    let name: String
    let email: String
    let avatar: Img_image
  }

  private(set) var profile: Profile?
  private(set) var loadFailed = false

  @ObservationIgnored private let query: WatchQuery<NoInput, ProfileScreen_Query>

  init() {
    query = WatchQuery(client: Container.shared.graphQLClient(), query: ProfileScreen_Query())
    keepObserving(while: self) { [weak self] in self?.sync() }
  }

  func refetch() {
    query.refetch()
  }

  private func sync() {
    let me = query.data?.me
    let error = query.error
    if let me {
      profile = Profile(name: me.name, email: me.email, avatar: me.avatar.fragments.img_image)
      loadFailed = false
    }
    if error != nil, me == nil, profile == nil {
      loadFailed = true
    }
  }
}
