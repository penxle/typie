import Core
import Editor
import FactoryKit
import Foundation

extension Container {
  @MainActor public var bottomChrome: Factory<BottomChrome> {
    self { BottomChrome() }.scope(.singleton)
  }

  @MainActor var editorResources: Factory<Task<EditorResources, any Error>> {
    self { Task { try await EditorResources.load() } }.scope(.singleton)
  }

  @MainActor var fontLoader: Factory<Task<FontLoader, any Error>> {
    self {
      let resources = self.editorResources()
      let session = URLSession(configuration: self.httpSessionConfiguration())
      let cache = URL.cachesDirectory.appending(path: "EditorFonts", directoryHint: .isDirectory)
      return Task {
        FontLoader(
          resources: try await resources.value,
          fetch: { url in
            let (data, response) = try await session.data(from: url)
            let status = (response as? HTTPURLResponse)?.statusCode ?? 0
            guard (200..<300).contains(status) else { throw HTTPError.status(status) }
            return data
          },
          cacheDirectory: cache)
      }
    }.scope(.singleton)
  }

  @MainActor var sites: Factory<SitesStore> {
    self { SitesStore() }.scope(.session)
  }

  @MainActor var liveUpdates: Factory<LiveUpdates> {
    self { LiveUpdates() }.scope(.session)
  }

  @MainActor var homeStore: Factory<HomeStore> {
    self { HomeStore() }
  }

  @MainActor var homeLayoutStore: Factory<HomeLayoutStore> {
    self { HomeLayoutStore() }.scope(.session)
  }

  @MainActor var recentDocumentsStore: Factory<RecentDocumentsStore> {
    self { RecentDocumentsStore() }
  }

  @MainActor var siteEntitiesStore: Factory<SiteEntitiesStore> {
    self { SiteEntitiesStore() }
  }

  @MainActor var homeTreeStore: Factory<HomeTreeStore> {
    self { HomeTreeStore() }.scope(.session)
  }

  @MainActor var entityCreator: Factory<EntityCreator> {
    self { EntityCreator() }.scope(.session)
  }

  @MainActor var userGoalModel: Factory<UserGoalModel> {
    self { UserGoalModel() }
  }

  @MainActor var profileModel: Factory<ProfileModel> {
    self { ProfileModel() }
  }

  @MainActor var emailLoginModel: ParameterFactory<@MainActor () -> Void, EmailLoginModel> {
    self { EmailLoginModel(onSuccess: $0) }
  }

  @MainActor var singleSignOnModel: ParameterFactory<@MainActor () -> Void, SingleSignOnModel> {
    self { SingleSignOnModel(onSuccess: $0) }
  }

  @MainActor var createSiteModel: Factory<CreateSiteModel> {
    self { CreateSiteModel() }
  }

  @MainActor var userGoalFormModel: ParameterFactory<UserGoalModel, UserGoalFormModel> {
    self { UserGoalFormModel(goal: $0) }
  }

  @MainActor var searchModel: Factory<SearchModel> {
    self { SearchModel() }
  }

  @MainActor var documentFontFamiliesModel: Factory<DocumentFontFamiliesModel> {
    self { DocumentFontFamiliesModel() }
  }
}

#if canImport(UIKit)

  extension Container {
    @MainActor var router: Factory<Router> {
      self { Router() }.scope(.session)
    }
  }

#endif
