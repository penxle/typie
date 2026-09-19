import FactoryKit

extension Container {
  @MainActor public var toast: Factory<TToastCenter> {
    self { TToastCenter() }.singleton
  }

  @MainActor public var dialog: Factory<TDialogCenter> {
    self { TDialogCenter() }.singleton
  }

  @MainActor public var theme: Factory<TThemeSettings> {
    self { TThemeSettings() }.singleton
  }
}
