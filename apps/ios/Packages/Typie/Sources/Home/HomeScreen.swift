#if canImport(UIKit)

  import Design
  import SwiftUI
  import UIKit

  public struct HomeScreen<Extra: View>: View {
    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }

    @FocusState private var fieldFocused: Bool
    @State private var scrollOffset: CGFloat = 0
    @State private var headingHeight: CGFloat = 0

    private let model: SearchModel
    private let search: HomeSearchState
    private let spaces: SpaceSwitcherModel
    private let onOpen: (SearchHit) -> Void
    private let extra: Extra

    public init(
      model: SearchModel, search: HomeSearchState, spaces: SpaceSwitcherModel,
      onOpen: @escaping (SearchHit) -> Void, @ViewBuilder extra: () -> Extra
    ) {
      self.model = model
      self.search = search
      self.spaces = spaces
      self.onOpen = onOpen
      self.extra = extra()
    }

    public var body: some View {
      ZStack(alignment: .top) {
        homeLayer
          .background(colors.surfaceCanvas.ignoresSafeArea())
          .opacity(search.isActive ? 0 : 1)
          .allowsHitTesting(!search.isActive)
        if search.isActive {
          searchLayer
            .background(colors.surfaceCanvas.ignoresSafeArea())
            .transition(
              .offset(y: Self.layerRise)
                .combined(with: .opacity.animation(reduceMotion ? nil : .easeOut(duration: 0.25))))
        }
      }
      .canvasBackground()
      .animation(transition, value: search.isActive)
      .onChange(of: search.isActive) { _, active in
        guard active else { return }
        Task { @MainActor in
          await Task.yield()
          guard search.isActive else { return }
          fieldFocused = true
        }
      }
    }

    private var transition: Animation? {
      reduceMotion
        ? nil
        : .spring(
          duration: HomeSearchState.transitionDuration, bounce: HomeSearchState.transitionBounce)
    }

    private var placeholder: String {
      SearchPrompt.text(spaceName: spaces.current?.name)
    }

    private var homeLayer: some View {
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          heading
          TSearchFieldButton(placeholder: placeholder, action: activate)
            .padding(.horizontal, 16)
            .padding(.top, 8)
            .padding(.bottom, 4)
          VStack(spacing: 12) {
            extra
          }
          .padding(.horizontal, 16)
          .padding(.vertical, 8)
        }
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity)
        .padding(.top, search.barHeight)
      }
      .onScrollGeometryChange(for: CGFloat.self) { geometry in
        geometry.contentOffset.y + geometry.contentInsets.top
      } action: { _, offset in
        scrollOffset = offset
        syncTitle()
      }
      .onChange(of: search.isActive) { _, _ in syncTitle() }
    }

    private var searchLayer: some View {
      VStack(spacing: 0) {
        HStack(spacing: 12) {
          TSearchField(
            text: textBinding, isFocused: $fieldFocused, placeholder: placeholder,
            onSubmit: { model.submit() })
          Button(action: deactivate) {
            TText("취소", style: TTypography.control, color: colors.textDefault)
          }
          .buttonStyle(.plain)
        }
        .padding(.horizontal, 16)
        .padding(.top, 8)
        .padding(.bottom, 4)
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity)
        ScrollView {
          SearchResults(model: model, onOpen: onOpen)
        }
        .scrollDismissesKeyboard(.interactively)
      }
      .onDisappear {
        fieldFocused = false
        if !search.isActive { model.setText("") }
      }
    }

    private var heading: some View {
      TText("홈", style: TTypography.hero, color: colors.textDefault)
        .padding(.horizontal, 16)
        .padding(.top, 8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .onGeometryChange(for: CGFloat.self) { proxy in
          proxy.size.height
        } action: { height in
          headingHeight = height
        }
    }

    private static var layerRise: CGFloat { 12 }

    private var textBinding: Binding<String> {
      Binding(get: { model.text }, set: { model.setText($0) })
    }

    private func activate() {
      search.isActive = true
    }

    private func deactivate() {
      search.isActive = false
      Task { @MainActor in
        guard !search.isActive else { return }
        fieldFocused = false
        UIApplication.shared.sendAction(
          #selector(UIResponder.resignFirstResponder), to: nil, from: nil, for: nil)
      }
    }

    private func syncTitle() {
      let visible = !search.isActive && scrollOffset >= headingHeight
      if search.titleVisible != visible {
        search.titleVisible = visible
      }
    }
  }

#endif
