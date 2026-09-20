import Foundation
import GraphQL

struct Site: Identifiable, Equatable, Sendable {
  let id: String
  let name: String
  let url: String
  let logo: Img_image?

  init(id: String, name: String, url: String, logo: Img_image?) {
    self.id = id
    self.name = name
    self.url = url
    self.logo = logo
  }
}
