#if canImport(UIKit)

  import Core
  import Design
  import SwiftUI

  public struct SearchResults: View {
    @Environment(\.theme) private var theme
    private var colors: TColors { theme.colors }

    private let model: SearchModel
    private let onOpen: (SearchHit) -> Void

    public init(model: SearchModel, onOpen: @escaping (SearchHit) -> Void) {
      self.model = model
      self.onOpen = onOpen
    }

    public var body: some View {
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

    private var recent: some View {
      VStack(alignment: .leading, spacing: 0) {
        TText("최근 검색", style: TTypography.section, color: colors.textMuted)
          .padding(.vertical, 8)
        if model.recentSearches.isEmpty {
          message("최근 검색어가 없어요")
        } else {
          TWrapLayout(spacing: 8) {
            ForEach(model.recentSearches, id: \.self) { keyword in
              chip(keyword)
            }
          }
        }
      }
      .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func chip(_ keyword: String) -> some View {
      HStack(spacing: 6) {
        Button {
          model.select(recent: keyword)
        } label: {
          TText(keyword, style: TTypography.control, color: colors.textDefault, maxLines: 1)
            .padding(.leading, 12)
            .padding(.vertical, 6)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        Button {
          model.remove(recent: keyword)
        } label: {
          TIcon(LucideIcon.x, size: 12, tint: colors.textHint, relativeTo: TTypography.control)
            .frame(minWidth: 24, minHeight: 24)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("삭제")
      }
      .padding(.trailing, 6)
      .frame(minHeight: 32)
      .background(colors.surfaceInset, in: TShapes.capsule)
    }

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

#endif
