import Core
import FactoryKit

extension Container {
  @MainActor public var bottomChrome: Factory<BottomChrome> {
    self { BottomChrome() }.scope(.singleton)
  }

  @MainActor var sites: Factory<SitesStore> {
    self { SitesStore() }.scope(.session)
  }

  @MainActor var homeStore: Factory<HomeStore> {
    self { HomeStore() }
  }

  @MainActor var recentDocumentsStore: Factory<RecentDocumentsStore> {
    self { RecentDocumentsStore() }
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
}

#if canImport(UIKit)

  extension Container {
    @MainActor var router: Factory<Router> {
      self { Router() }.scope(.session)
    }
  }

#endif
