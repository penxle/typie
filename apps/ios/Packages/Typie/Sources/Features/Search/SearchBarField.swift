#if canImport(UIKit)

  import Design
  import FactoryKit
  import SwiftUI

  @MainActor
  struct SearchBarField: View {
    @FocusState private var focused: Bool

    private let session: SearchSession
    private let sites = Container.shared.sites()

    init(session: SearchSession) {
      self.session = session
    }

    var body: some View {
      TSearchField(
        text: Binding(get: { session.model.text }, set: { session.model.setText($0) }),
        isFocused: $focused,
        placeholder: SearchSession.searchPrompt(siteName: sites.current?.name),
        onSubmit: { session.model.submit() }
      )
      .onChange(of: session.focusRequest) { _, _ in focused = true }
      .onChange(of: session.blurRequest) { _, _ in focused = false }
    }
  }

#endif
