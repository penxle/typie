import Core
import FactoryKit

extension Container {
  @MainActor var sites: Factory<SitesStore> {
    self { SitesStore() }.scope(.session)
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
