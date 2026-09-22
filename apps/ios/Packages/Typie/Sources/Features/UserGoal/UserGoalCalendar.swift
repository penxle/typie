import Core
import Design
import SwiftUI

@MainActor
struct UserGoalCalendar: View {
  static let rowHeight: CGFloat = 72
  static let headerHeight: CGFloat = 32 + 6 + 16 + 2

  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }

  private let month: UserGoalMonth
  private let selected: KSTDay
  private let streak: Int
  @Binding private var expanded: Bool
  private let onSelect: (KSTDay) -> Void

  init(
    month: UserGoalMonth, selected: KSTDay, streak: Int, expanded: Binding<Bool>,
    onSelect: @escaping (KSTDay) -> Void
  ) {
    self.month = month
    self.selected = selected
    self.streak = streak
    _expanded = expanded
    self.onSelect = onSelect
  }

  static var expandAnimation: Animation {
    .timingCurve(0.23, 1, 0.32, 1, duration: 0.32)
  }

  private var keptRow: Int { month.row(of: selected) ?? month.row(of: month.today) ?? 0 }

  var body: some View {
    VStack(spacing: 0) {
      header
      weekdays
      VStack(spacing: 0) {
        ForEach(Array(month.rows.enumerated()), id: \.offset) { index, row in
          let visible = expanded || index == keptRow
          HStack(spacing: 4) {
            ForEach(row, id: \.day) { cell in
              self.cell(cell)
            }
          }
          .frame(height: visible ? Self.rowHeight : 0)
          .opacity(visible ? 1 : 0)
          .clipped()
          .accessibilityHidden(!visible)
        }
      }
      .animation(reduceMotion ? nil : Self.expandAnimation, value: expanded)
      .animation(reduceMotion ? nil : Self.expandAnimation, value: keptRow)
    }
  }

  private var header: some View {
    HStack(spacing: 8) {
      TText(
        expanded ? month.monthLabel : month.weekLabel(of: selected), style: TTypography.section,
        color: colors.textMuted)
      Spacer(minLength: 0)
      if streak > 0 {
        TText(
          "달성 연속 \(streak)일", style: TTypography.caption, color: colors.textHint,
          monospacedDigit: true)
      }
      Button {
        expanded.toggle()
      } label: {
        TIcon(
          LucideIcon.chevronDown, size: 20, tint: colors.textMuted, relativeTo: TTypography.label
        )
        .rotationEffect(.degrees(expanded ? 180 : 0))
        .animation(reduceMotion ? nil : Self.expandAnimation, value: expanded)
        .frame(width: 32, height: 32)
        .contentShape(Rectangle())
      }
      .buttonStyle(TPressLook(scale: 0.94))
      .padding(.trailing, -8)
      .accessibilityLabel(expanded ? "월 달력 접기" : "월 달력 펼치기")
    }
    .frame(height: 32)
    .padding(.bottom, 6)
  }

  private var weekdays: some View {
    HStack(spacing: 4) {
      ForEach(Array(UserGoalFormat.weekdayNames.enumerated()), id: \.offset) { index, name in
        let isToday = !expanded && index == month.todayColumn
        TText(
          name, style: TTypography.meta, color: isToday ? colors.textDefault : colors.textHint
        )
        .fontWeight(isToday ? .bold : nil)
        .frame(maxWidth: .infinity)
      }
    }
    .padding(.bottom, 2)
  }

  private func cell(_ cell: UserGoalMonthCell) -> some View {
    let isSelected = cell.day == selected
    let isToday = cell.day == month.today
    return Button {
      guard cell.isSelectable else { return }
      onSelect(cell.day)
    } label: {
      VStack(spacing: 4) {
        mark(cell)
          .frame(width: 32, height: 32)
        TText(
          "\(cell.day.day)", style: TTypography.caption,
          color: isSelected
            ? colors.surfaceCanvas : (isToday ? colors.textDefault : colors.textHint),
          monospacedDigit: true
        )
        .fontWeight(isSelected || isToday ? .bold : nil)
        .frame(minWidth: 24, minHeight: 24)
        .padding(.horizontal, 4)
        .background(isSelected ? colors.accentDefault : .clear, in: Capsule())
        .opacity(cell.state == .out ? 0.4 : 1)
      }
      .frame(maxWidth: .infinity)
      .frame(height: Self.rowHeight)
      .contentShape(Rectangle())
    }
    .buttonStyle(TPressLook(scale: 0.96))
    .disabled(!cell.isSelectable)
    .accessibilityHidden(cell.state == .out)
    .accessibilityAddTraits(isSelected ? .isSelected : [])
  }

  @ViewBuilder
  private func mark(_ cell: UserGoalMonthCell) -> some View {
    switch cell.state {
    case .achieved:
      Circle().fill(colors.successDefault)
        .overlay(TIcon(LucideIcon.check, size: 16, tint: colors.textOnSuccess))
    case .missed:
      Circle().fill(colors.borderDefault)
    case .today:
      if cell.progress >= 1 {
        Circle().fill(colors.successDefault)
          .overlay(TIcon(LucideIcon.check, size: 16, tint: colors.textOnSuccess))
      } else {
        TProgressRing(progress: cell.progress, state: .under, size: 32)
      }
    case .future:
      Circle().strokeBorder(colors.borderHairline, lineWidth: 1.5)
    case .noGoal, .out:
      Color.clear
    }
  }
}
