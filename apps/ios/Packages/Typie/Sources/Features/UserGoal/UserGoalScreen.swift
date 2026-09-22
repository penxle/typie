#if canImport(UIKit)

  import Core
  import Design
  import SwiftUI

  @MainActor
  struct UserGoalScreen: View {
    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }

    private let model: UserGoalModel
    private let onOpenDocument: (String) -> Void

    @State private var expanded = false
    @State private var dayHeight: CGFloat = 0
    private static let labelHeight: CGFloat = 44
    private static let stageGap: CGFloat = 24

    init(model: UserGoalModel, onOpenDocument: @escaping (String) -> Void) {
      self.model = model
      self.onOpenDocument = onOpenDocument
    }

    var body: some View {
      Group {
        if model.hasData, let month = model.grid(for: model.displayedWeek),
          let day = model.selected
        {
          content(month: month, day: day)
        } else if model.loadFailed {
          RetryPrompt { model.refetch() }
            .padding(.horizontal, 16)
        } else {
          Color.clear
        }
      }
      .canvasBackground()
      .onAppear { model.refetch() }
    }

    private func content(month: UserGoalMonth, day: UserGoalDay) -> some View {
      GeometryReader { proxy in
        let rowCount = CGFloat(UserGoalCalendar.expandedRows)
        let calendarHeight =
          UserGoalCalendar.headerHeight + (expanded ? rowCount : 1) * UserGoalCalendar.rowHeight
        let top: CGFloat = 12
        let stageTop = top + calendarHeight
        let roomAboveLabel = proxy.size.height - stageTop - Self.labelHeight
        let stageHeight = expanded ? max(roomAboveLabel, dayHeight + Self.stageGap * 2) : 0
        let ringCenter = expanded ? stageTop + stageHeight / 2 : proxy.size.height / 2
        let spacerHeight = expanded ? stageHeight : roomAboveLabel
        ScrollViewReader { scroller in
          ScrollView {
            ZStack(alignment: .top) {
              VStack(spacing: 0) {
                UserGoalCalendar(
                  displayedWeek: model.displayedWeek, anchor: model.anchor,
                  selected: model.selectedDay, today: model.today,
                  streak: model.state?.streaks.current ?? 0, expanded: $expanded,
                  weeks: model.weeks, monthPages: model.monthPages,
                  grid: { model.grid(for: $0) }, onSelect: { model.select($0) },
                  onShow: { model.show(week: $0) },
                  onCollapsed: { model.settleDisplayedWeek() }
                )
                .padding(.top, top)
                Color.clear.frame(height: max(0, spacerHeight))
                documentsLabel { scroller.scrollTo("documents", anchor: .top) }
                if hasDocumentsSection {
                  UserGoalDocumentsView(
                    documents: model.documents, failed: model.documentsFailed
                  ) { onOpenDocument($0.entityId) }
                  .id("documents")
                  .padding(.top, proxy.safeAreaInsets.bottom)
                  .padding(.bottom, 40)
                }
              }
              .padding(.horizontal, 16)
              .frame(maxWidth: 600)
              .frame(maxWidth: .infinity)
              UserGoalDayView(day: day)
                .onGeometryChange(for: CGFloat.self, of: { $0.size.height }) { dayHeight = $0 }
                .position(x: proxy.size.width / 2, y: ringCenter)
                .frame(height: 0)
                .animation(reduceMotion ? nil : UserGoalCalendar.expandAnimation, value: expanded)
                .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: day)
            }
          }
        }
      }
    }

    private var hasDocumentsSection: Bool {
      !model.documents.isEmpty || model.documentsFailed
    }

    private var documentsLabelText: String {
      if model.documentsFailed { return "이 날 작성한 문서" }
      if model.documents.isEmpty { return "이 날 작성한 문서가 없어요" }
      return "이 날 작성한 문서 \(model.documents.count)개"
    }

    @ViewBuilder
    private func documentsLabel(action: @escaping () -> Void) -> some View {
      if hasDocumentsSection {
        Button {
          withAnimation(reduceMotion ? nil : UserGoalCalendar.expandAnimation) { action() }
        } label: {
          HStack(spacing: 6) {
            TText(
              documentsLabelText, style: TTypography.caption, color: colors.textHint,
              monospacedDigit: true)
            TIcon(
              LucideIcon.chevronRight, size: 14, tint: colors.textHint,
              relativeTo: TTypography.caption
            )
            .rotationEffect(.degrees(90))
          }
          .frame(maxWidth: .infinity, minHeight: Self.labelHeight)
          .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("이 날 작성한 문서로 이동")
      } else {
        TText(documentsLabelText, style: TTypography.caption, color: colors.textHint)
          .frame(maxWidth: .infinity, minHeight: Self.labelHeight)
      }
    }
  }

#endif
