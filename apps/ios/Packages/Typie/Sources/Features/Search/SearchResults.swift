#if canImport(UIKit)

  import Core
  import Design
  import SwiftUI

  struct SearchResults: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let model: SearchModel
    private let editingRecent: Binding<Bool>
    private let onOpen: (SearchHit) -> Void

    init(model: SearchModel, editingRecent: Binding<Bool>, onOpen: @escaping (SearchHit) -> Void) {
      self.model = model
      self.editingRecent = editingRecent
      self.onOpen = onOpen
    }

    var body: some View {
      VStack(spacing: 12) {
        switch model.content {
        case .recent: recent
        case .pending: EmptyView()
        case .failed: message("검색 중 오류가 발생했어요")
        case .empty: message("검색 결과가 없어요")
        case .results(let hits): results(hits)
        }
      }
      .padding(.horizontal, 16)
      .padding(.vertical, 8)
      .frame(maxWidth: 600)
      .frame(maxWidth: .infinity)
    }

    private func message(_ text: String) -> some View {
      TText(text, style: TTypography.control, color: colors.textMuted)
        .frame(maxWidth: .infinity)
        .padding(.vertical, 24)
    }

    @ViewBuilder
    private var recent: some View {
      if !model.recentSearches.isEmpty {
        RecentSearches(model: model, editing: editingRecent)
          .padding(.bottom, Self.recentBottomGap)
      }
    }

    private static let recentBottomGap: CGFloat = 16

    private func results(_ hits: [SearchHit]) -> some View {
      VStack(spacing: 0) {
        ForEach(Array(hits.enumerated()), id: \.element.id) { index, hit in
          Button {
            model.didOpen(hit)
            onOpen(hit)
          } label: {
            row(hit)
          }
          .buttonStyle(.plain)
          if index < hits.count - 1 {
            Rectangle().fill(colors.borderHairline).frame(height: 1)
          }
        }
      }
    }

    @ViewBuilder
    private func row(_ hit: SearchHit) -> some View {
      switch hit {
      case .document(let document):
        EntityRowView(
          icon: document.icon, path: document.path,
          trailing: document.updatedAt.map { timeAgo($0) }, title: document.title,
          snippet: document.preview)
      case .folder(let folder):
        EntityRowView(
          icon: folder.icon, path: folder.path, trailing: folder.summary, title: folder.title)
      }
    }
  }

  private struct RecentSearches: View {
    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }

    @Binding private var editing: Bool
    private let model: SearchModel

    init(model: SearchModel, editing: Binding<Bool>) {
      self.model = model
      _editing = editing
    }

    var body: some View {
      VStack(alignment: .leading, spacing: 0) {
        HStack {
          TText("최근 검색", style: TTypography.section, color: colors.textMuted)
          Spacer()
          if editing {
            Button {
              editing = false
            } label: {
              TText("완료", style: TTypography.section, color: colors.textDefault)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .transition(.opacity)
          }
        }
        .padding(.vertical, 8)
        TWrapLayout(spacing: 8) {
          ForEach(model.recentSearches, id: \.self) { keyword in
            chip(keyword)
              .transition(appearance)
          }
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
      .animation(modeAnimation, value: editing)
      .onDisappear { editing = false }
    }

    private var modeAnimation: Animation {
      reduceMotion ? .linear(duration: 0.15) : .smooth(duration: 0.25)
    }

    private var removalAnimation: Animation {
      reduceMotion ? .linear(duration: 0.15) : .smooth(duration: 0.3)
    }

    private var appearance: AnyTransition {
      reduceMotion ? .opacity : .scale(scale: 0.9).combined(with: .opacity)
    }

    private func remove(_ keyword: String) {
      withAnimation(removalAnimation) { model.remove(recent: keyword) }
    }

    private func chip(_ keyword: String) -> some View {
      Button {
        guard !editing else { return }
        model.select(recent: keyword)
      } label: {
        TText(keyword, style: TTypography.control, color: colors.textDefault, maxLines: 1)
          .padding(.horizontal, 12)
          .padding(.vertical, 6)
          .frame(minHeight: 32)
          .background(colors.surfaceInset, in: TShapes.capsule)
          .contentShape(TShapes.capsule)
      }
      .buttonStyle(.plain)
      .simultaneousGesture(LongPressGesture().onEnded { _ in editing = true })
      .overlay(alignment: .topTrailing) {
        if editing {
          Button {
            remove(keyword)
          } label: {
            TIcon(LucideIcon.x, size: 10, tint: colors.surfaceCanvas)
              .frame(width: Self.removeSide, height: Self.removeSide)
              .background(colors.textDefault, in: Circle())
              .frame(minWidth: 24, minHeight: 24)
              .contentShape(Circle())
          }
          .buttonStyle(.plain)
          .accessibilityLabel("삭제")
          .offset(x: Self.removeSide / 3, y: -Self.removeSide / 3)
          .transition(appearance)
        }
      }
      .accessibilityAction(named: "삭제") { remove(keyword) }
    }

    private static let removeSide: CGFloat = 18
  }

#endif
