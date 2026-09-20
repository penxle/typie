#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct SearchScreen: View {
    private let model: SearchModel
    private let keyboardInset: CGFloat
    private let onOpen: (SearchHit) -> Void
    private let onDismissKeyboard: () -> Void
    @State private var editingRecent = false

    init(
      model: SearchModel, keyboardInset: CGFloat, onOpen: @escaping (SearchHit) -> Void,
      onDismissKeyboard: @escaping () -> Void
    ) {
      self.model = model
      self.keyboardInset = keyboardInset
      self.onOpen = onOpen
      self.onDismissKeyboard = onDismissKeyboard
    }

    private var alignment: Alignment {
      switch model.content {
      case .recent: .bottom
      case .empty, .failed: .center
      case .pending, .results: .top
      }
    }

    var body: some View {
      GeometryReader { proxy in
        ScrollView {
          SearchResults(model: model, editingRecent: $editingRecent, onOpen: onOpen)
            .frame(minHeight: proxy.size.height, alignment: alignment)
            .contentShape(Rectangle())
            .onTapGesture { interact() }
        }
        .onScrollPhaseChange { _, phase in
          if phase == .interacting { interact() }
        }
      }
      .scrollDismissesKeyboard(.immediately)
      .safeAreaPadding(.bottom, keyboardInset)
      .ignoresSafeArea(.container, edges: .bottom)
      .canvasBackground()
    }

    private func interact() {
      if editingRecent {
        editingRecent = false
        return
      }
      model.didInteract()
      onDismissKeyboard()
    }
  }

#endif
