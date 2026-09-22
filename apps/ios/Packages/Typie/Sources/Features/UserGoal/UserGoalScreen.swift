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
    private let onEdit: () -> Void
    private let onOpenDocument: (String) -> Void

    @State private var expanded = false

    init(
      model: UserGoalModel, onEdit: @escaping () -> Void,
      onOpenDocument: @escaping (String) -> Void
    ) {
      self.model = model
      self.onEdit = onEdit
      self.onOpenDocument = onOpenDocument
    }

    var body: some View {
      Group {
        if model.hasData, model.hasGoal, let month = model.state?.month, let day = model.selected {
          content(month: month, day: day)
        } else if model.hasData {
          callout
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
        let rowCount = CGFloat(month.rows.count)
        let calendarHeight =
          UserGoalCalendar.headerHeight + (expanded ? rowCount : 1) * UserGoalCalendar.rowHeight
        let top: CGFloat = 12
        let stageTop = top + calendarHeight
        let ringCenter =
          expanded ? stageTop + (proxy.size.height - stageTop) / 2 : proxy.size.height / 2
        ScrollViewReader { scroller in
          ScrollView {
            ZStack(alignment: .top) {
              VStack(spacing: 0) {
                UserGoalCalendar(
                  month: month, selected: model.selectedDay,
                  streak: model.state?.streaks.current ?? 0, expanded: $expanded
                ) { model.select($0) }
                .padding(.top, top)
                Color.clear.frame(height: max(0, proxy.size.height - top - calendarHeight - 40))
                UserGoalDocumentsView(
                  documents: model.documents, failed: model.documentsFailed
                ) { onOpenDocument($0.entityId) }
                .id("documents")
                .padding(.bottom, 40)
              }
              UserGoalDayView(day: day)
                .position(x: proxy.size.width / 2, y: ringCenter)
                .frame(height: 0)
                .animation(reduceMotion ? nil : UserGoalCalendar.expandAnimation, value: expanded)
                .animation(reduceMotion ? nil : .easeOut(duration: 0.15), value: day)
            }
            .padding(.horizontal, 16)
            .frame(maxWidth: 600)
            .frame(maxWidth: .infinity)
          }
          .overlay(alignment: .bottom) {
            hintBar { scroller.scrollTo("documents", anchor: .top) }
          }
        }
      }
    }

    private func hintBar(action: @escaping () -> Void) -> some View {
      Button {
        withAnimation(reduceMotion ? nil : .easeOut(duration: 0.3)) { action() }
      } label: {
        HStack(spacing: 6) {
          TText(
            model.documents.isEmpty ? "쓴 글" : "쓴 글 \(model.documents.count)개",
            style: TTypography.caption, color: colors.textHint, monospacedDigit: true)
          TIcon(
            LucideIcon.chevronRight, size: 14, tint: colors.textHint,
            relativeTo: TTypography.caption
          )
          .rotationEffect(.degrees(90))
        }
        .frame(maxWidth: .infinity, minHeight: 44)
        .background(
          LinearGradient(
            colors: [colors.surfaceCanvas.opacity(0), colors.surfaceCanvas],
            startPoint: .top, endPoint: UnitPoint(x: 0.5, y: 0.4))
        )
        .contentShape(Rectangle())
      }
      .buttonStyle(.plain)
      .accessibilityLabel("쓴 글로 이동")
    }

    private var callout: some View {
      VStack(alignment: .leading, spacing: 16) {
        TText(
          "매일 쓸 글자 수를 정해 보세요. 달성한 날이 기록으로 쌓이고, 연속 달성 일수도 볼 수 있어요.",
          style: TTypography.text, color: colors.textMuted)
        TButton("일일 목표 정하기") { onEdit() }
      }
      .padding(.horizontal, 16)
      .padding(.top, 12)
      .frame(maxWidth: 600, alignment: .leading)
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }
  }

#endif
