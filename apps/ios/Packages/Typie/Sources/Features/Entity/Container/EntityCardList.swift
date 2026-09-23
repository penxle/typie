import Foundation

enum EntityCardBlock: Equatable, Sendable, Identifiable {
  case documents([EntityDocumentItem])
  case folder(EntityFolderItem)
  case divider(id: String)

  var id: String {
    switch self {
    case .documents(let documents): "documents-\(documents[0].entityId)"
    case .folder(let folder): folder.entityId
    case .divider(let id): id
    }
  }
}

enum EntityCardListRules {
  static func blocks(_ items: [EntityContainerItem]) -> [EntityCardBlock] {
    var blocks: [EntityCardBlock] = []
    var run: [EntityDocumentItem] = []
    func flush() {
      if !run.isEmpty {
        blocks.append(.documents(run))
        run = []
      }
    }
    for item in items {
      switch item {
      case .entity(.document(let document)):
        run.append(document)
      case .entity(.folder(let folder)):
        flush()
        blocks.append(.folder(folder))
      case .divider(let id):
        flush()
        blocks.append(.divider(id: id))
      }
    }
    flush()
    return blocks
  }
}

#if canImport(UIKit)

  import Core
  import Design
  import SwiftUI

  struct EntityCardList: View {
    @Environment(\.theme) private var theme

    private let items: [EntityContainerItem]
    private let now: Date
    private let onOpenDocument: (String) -> Void
    private let onOpenFolder: (EntityFolderItem) -> Void

    init(
      items: [EntityContainerItem], now: Date, onOpenDocument: @escaping (String) -> Void,
      onOpenFolder: @escaping (EntityFolderItem) -> Void
    ) {
      self.items = items
      self.now = now
      self.onOpenDocument = onOpenDocument
      self.onOpenFolder = onOpenFolder
    }

    var body: some View {
      VStack(spacing: 12) {
        ForEach(EntityCardListRules.blocks(items)) { block in
          switch block {
          case .documents(let documents):
            documentCard(documents)
          case .folder(let folder):
            EntityFolderTile(folder: folder) { onOpenFolder(folder) }
          case .divider:
            EntityDividerCard()
          }
        }
      }
    }

    private func documentCard(_ documents: [EntityDocumentItem]) -> some View {
      VStack(spacing: 0) {
        ForEach(Array(documents.enumerated()), id: \.element.entityId) { index, document in
          if index > 0 {
            Rectangle()
              .fill(theme.colors.borderHairline)
              .frame(height: 1)
              .padding(.leading, 16)
          }
          EntityDocumentRow(document: document, now: now) { onOpenDocument(document.entityId) }
        }
      }
      .background(theme.colors.surfaceDefault)
      .clipShape(TShapes.squircle(TShapes.md))
    }
  }

  struct EntityDocumentRow: View {
    @Environment(\.theme) private var theme
    private var titleMetrics = TTextMetrics(TTypography.label)

    private let document: EntityDocumentItem
    private let now: Date
    private let onOpen: () -> Void

    init(document: EntityDocumentItem, now: Date, onOpen: @escaping () -> Void) {
      self.document = document
      self.now = now
      self.onOpen = onOpen
    }

    var body: some View {
      let colors = theme.colors
      let appearance = EntityIcon.appearance(document.icon, colors: colors)
      Button(action: onOpen) {
        VStack(alignment: .leading, spacing: 8) {
          HStack(spacing: 8) {
            TIcon(appearance.icon, size: 16, tint: appearance.tint, relativeTo: TTypography.label)
            title
              .frame(maxWidth: .infinity, alignment: .leading)
            if let updatedAt = document.updatedAt {
              TText(
                timeAgo(updatedAt, now: now), style: TTypography.caption, color: colors.textHint,
                maxLines: 1
              )
              .layoutPriority(1)
            }
          }
          TText(
            EntityText.excerpt(document.excerpt), style: TTypography.detail,
            color: colors.textHint, maxLines: 1
          )
          .padding(.leading, 24)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
      }
      .buttonStyle(TPressEffectStyle(TPressLook()))
      .accessibilityElement(children: .combine)
    }

    @ViewBuilder private var title: some View {
      if let subtitle {
        (Text(document.title).foregroundStyle(theme.colors.textDefault)
          + Text(" · ").foregroundStyle(theme.colors.textHint)
          + Text(subtitle).foregroundStyle(theme.colors.textHint))
          .font(titleMetrics.style.font)
          .lineLimit(1)
          .lineSpacing(titleMetrics.extraLineSpacing)
          .padding(.vertical, titleMetrics.extraLineSpacing / 2)
      } else {
        TText(
          document.title, style: TTypography.label, color: theme.colors.textDefault, maxLines: 1)
      }
    }

    private var subtitle: String? {
      guard let subtitle = document.subtitle,
        !subtitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
      else { return nil }
      return subtitle
    }
  }

  struct EntityFolderTile: View {
    @Environment(\.theme) private var theme

    private let folder: EntityFolderItem
    private let onOpen: () -> Void

    init(folder: EntityFolderItem, onOpen: @escaping () -> Void) {
      self.folder = folder
      self.onOpen = onOpen
    }

    var body: some View {
      let colors = theme.colors
      let appearance = EntityIcon.appearance(folder.icon, colors: colors)
      Button(action: onOpen) {
        HStack(spacing: 12) {
          TIcon(appearance.icon, size: 18, tint: appearance.tint, relativeTo: TTypography.label)
            .frame(width: 36, height: 36)
            .background(colors.surfaceInset, in: TShapes.squircle(TShapes.sm))
          TText(folder.title, style: TTypography.label, color: colors.textDefault, maxLines: 1)
            .frame(maxWidth: .infinity, alignment: .leading)
          HStack(spacing: 6) {
            TText(
              Formatting.comma(folder.folderCount + folder.documentCount),
              style: TTypography.detail, color: colors.textHint, monospacedDigit: true)
            TIcon(
              LucideIcon.chevronRight, size: 16, tint: colors.textHint,
              relativeTo: TTypography.label)
          }
        }
        .padding(.horizontal, 12)
        .frame(maxWidth: .infinity, minHeight: 60, alignment: .leading)
        .background(colors.surfaceDefault, in: TShapes.squircle(TShapes.md))
        .contentShape(Rectangle())
      }
      .buttonStyle(TPressEffectStyle(TPressLook()))
      .accessibilityElement(children: .combine)
      .accessibilityValue("항목 \(folder.folderCount + folder.documentCount)개")
    }
  }

  struct EntityDividerCard: View {
    @Environment(\.theme) private var theme

    var body: some View {
      Rectangle()
        .fill(theme.colors.borderDefault)
        .frame(height: 1)
        .padding(.horizontal, 16)
        .frame(maxWidth: .infinity, minHeight: 60)
        .background(theme.colors.surfaceDefault, in: TShapes.squircle(TShapes.md))
        .accessibilityHidden(true)
    }
  }

#endif
