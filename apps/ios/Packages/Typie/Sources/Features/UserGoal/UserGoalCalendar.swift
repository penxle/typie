import Core
import Design
import SwiftUI

@MainActor
struct UserGoalCalendar: View {
  nonisolated static let rowHeight: CGFloat = 72
  nonisolated static let headerHeight: CGFloat = 32 + 6 + 16 + 2
  nonisolated static let expandedRows = UserGoalMonth.rowCount
  static var expandAnimation: Animation { .spring(duration: 0.3, bounce: 0) }
  private static let dimmedOpacity = 0.4

  @Environment(\.theme) private var theme
  @Environment(\.accessibilityReduceMotion) private var reduceMotion
  private var colors: TColors { theme.colors }

  private let displayedWeek: KSTDay
  private let anchor: KSTDay
  private let selected: KSTDay
  private let today: KSTDay
  private let streak: Int
  @Binding private var expanded: Bool
  private let weeks: [KSTDay]
  private let monthPages: [KSTDay]
  private let grid: (KSTDay) -> UserGoalMonth?
  private let onSelect: (KSTDay) -> Void
  private let onShow: (KSTDay) -> Void
  private let onCollapsed: () -> Void
  @State private var transitioning = false
  @State private var showsMonthPages: Bool

  init(
    displayedWeek: KSTDay, anchor: KSTDay, selected: KSTDay, today: KSTDay, streak: Int,
    expanded: Binding<Bool>, weeks: [KSTDay], monthPages: [KSTDay],
    grid: @escaping (KSTDay) -> UserGoalMonth?, onSelect: @escaping (KSTDay) -> Void,
    onShow: @escaping (KSTDay) -> Void, onCollapsed: @escaping () -> Void
  ) {
    self.displayedWeek = displayedWeek
    self.anchor = anchor
    self.selected = selected
    self.today = today
    self.streak = streak
    _expanded = expanded
    self.weeks = weeks
    self.monthPages = monthPages
    self.grid = grid
    self.onSelect = onSelect
    self.onShow = onShow
    self.onCollapsed = onCollapsed
    _showsMonthPages = State(initialValue: expanded.wrappedValue)
  }

  private var pagerHeight: CGFloat {
    (expanded ? CGFloat(Self.expandedRows) : 1) * Self.rowHeight
  }

  private var displayedPage: UserGoalMonth? { grid(displayedWeek) }

  var body: some View {
    VStack(spacing: 0) {
      header
      weekdays
      CalendarPager(
        pages: showsMonthPages ? monthPages : weeks, selection: displayedWeek, onShow: onShow
      ) {
        week in
        if let page = grid(week) {
          let kept = keptRow(of: page, week: week)
          rows(of: page, only: showsMonthPages || transitioning ? nil : kept)
            .modifier(
              Unfold(offset: expanded ? 0 : -CGFloat(kept) * Self.rowHeight, height: pagerHeight))
        }
      }
      .id(showsMonthPages)
      .frame(height: pagerHeight, alignment: .top)
      .modifier(EdgeFade(height: pagerHeight, background: colors.surfaceCanvas))
      .allowsHitTesting(!transitioning)
    }
  }

  private func keptRow(of page: UserGoalMonth, week: KSTDay) -> Int {
    if week == displayedWeek, let row = page.row(of: anchor) { return row }
    return page.row(of: week) ?? 0
  }

  private func toggle() {
    let opening = !expanded
    transitioning = true
    withAnimation(reduceMotion ? nil : Self.expandAnimation) {
      expanded = opening
    } completion: {
      guard expanded == opening else { return }
      withoutAnimation {
        showsMonthPages = opening
        transitioning = false
      }
      if !opening { onCollapsed() }
    }
  }

  private func withoutAnimation(_ change: () -> Void) {
    var transaction = Transaction()
    transaction.disablesAnimations = true
    withTransaction(transaction, change)
  }

  private func rows(of page: UserGoalMonth, only visibleRow: Int?) -> some View {
    VStack(spacing: 0) {
      ForEach(Array(page.rows.enumerated()), id: \.offset) { index, row in
        if visibleRow.map({ $0 == index }) ?? true, row.contains(where: \.inMonth) {
          HStack(spacing: 4) {
            ForEach(row, id: \.day) { cell in
              self.cell(cell, in: page)
            }
          }
          .frame(height: Self.rowHeight)
        } else {
          Color.clear.frame(height: Self.rowHeight)
        }
      }
    }
  }

  private var header: some View {
    HStack(spacing: 8) {
      TText(
        expanded ? displayedPage?.monthLabel ?? "" : displayedPage?.weekLabel(of: anchor) ?? "",
        style: TTypography.section, color: colors.textMuted)
      Spacer(minLength: 0)
      if streak > 0 {
        TText(
          "달성 연속 \(streak)일", style: TTypography.caption, color: colors.textHint,
          monospacedDigit: true)
      }
      Button {
        toggle()
      } label: {
        TIcon(
          LucideIcon.chevronDown, size: 20, tint: colors.textMuted, relativeTo: TTypography.label
        )
        .rotationEffect(.degrees(expanded ? 180 : 0))
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
    let todayInWeek = !expanded && UserGoalMonth.weekStart(of: today) == displayedWeek
    return HStack(spacing: 4) {
      ForEach(Array(UserGoalFormat.weekdayNames.enumerated()), id: \.offset) { index, name in
        let isToday = todayInWeek && index == today.weekday - 1
        TText(
          name, style: TTypography.meta, color: isToday ? colors.textDefault : colors.textHint
        )
        .fontWeight(isToday ? .bold : nil)
        .frame(maxWidth: .infinity)
      }
    }
    .padding(.bottom, 2)
  }

  private func cell(_ cell: UserGoalMonthCell, in page: UserGoalMonth) -> some View {
    let isSelected = cell.day == selected
    let isToday = cell.day == today
    let dimmed = expanded && !cell.inMonth
    let selectable = cell.isSelectable && !dimmed
    return Button {
      guard selectable else { return }
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
      }
      .opacity(dimmed ? Self.dimmedOpacity : 1)
      .frame(maxWidth: .infinity)
      .frame(height: Self.rowHeight)
      .contentShape(Rectangle())
    }
    .buttonStyle(TPressLook(scale: 0.96))
    .disabled(!selectable)
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
      TProgressRing(progress: cell.progress, state: .under, size: 32)
    case .today:
      if cell.progress >= 1 {
        Circle().fill(colors.successDefault)
          .overlay(TIcon(LucideIcon.check, size: 16, tint: colors.textOnSuccess))
      } else {
        TProgressRing(progress: cell.progress, state: .under, size: 32)
      }
    case .future:
      Circle().strokeBorder(colors.borderHairline, lineWidth: 1.5)
    case .noGoal:
      TProgressRing(progress: 0, state: .noGoal, size: 32)
    case .out:
      Color.clear
    }
  }
}

nonisolated private struct Unfold: ViewModifier, Animatable {
  var offset: CGFloat
  var height: CGFloat

  var animatableData: AnimatablePair<CGFloat, CGFloat> {
    get { AnimatablePair(offset, height) }
    set {
      offset = newValue.first
      height = newValue.second
    }
  }

  func body(content: Content) -> some View {
    content
      .offset(y: offset)
      .frame(height: height, alignment: .top)
  }
}

nonisolated private struct EdgeFade: ViewModifier, Animatable {
  private static let maxFade: CGFloat = 20
  var height: CGFloat
  let background: Color

  var animatableData: CGFloat {
    get { height }
    set { height = newValue }
  }

  private var fade: CGFloat {
    let collapsed = UserGoalCalendar.rowHeight
    let expanded = CGFloat(UserGoalCalendar.expandedRows) * UserGoalCalendar.rowHeight
    let travel = min(height - collapsed, expanded - height)
    return max(0, min(Self.maxFade, travel / 2))
  }

  func body(content: Content) -> some View {
    content
      .clipped()
      .overlay(alignment: .top) {
        LinearGradient(colors: [background, .clear], startPoint: .top, endPoint: .bottom)
          .frame(height: fade)
          .allowsHitTesting(false)
      }
      .overlay(alignment: .bottom) {
        LinearGradient(colors: [.clear, background], startPoint: .top, endPoint: .bottom)
          .frame(height: fade)
          .allowsHitTesting(false)
      }
  }
}

@MainActor
private struct CalendarPager<Page: View>: View {
  private let pages: [KSTDay]
  private let selection: KSTDay
  private let onShow: (KSTDay) -> Void
  private let page: (KSTDay) -> Page
  @State private var current: KSTDay?

  init(
    pages: [KSTDay], selection: KSTDay, onShow: @escaping (KSTDay) -> Void,
    @ViewBuilder page: @escaping (KSTDay) -> Page
  ) {
    self.pages = pages
    self.selection = selection
    self.onShow = onShow
    self.page = page
    _current = State(initialValue: selection)
  }

  var body: some View {
    TabView(selection: $current) {
      ForEach(pages, id: \.self) { start in
        page(start)
          .frame(maxHeight: .infinity, alignment: .top)
          .tag(Optional(start))
      }
    }
    .modifier(PagedTabs())
    .onChange(of: selection) { _, selection in
      guard current != selection else { return }
      var transaction = Transaction()
      transaction.disablesAnimations = true
      withTransaction(transaction) { current = selection }
    }
    .onChange(of: current) { _, landed in
      guard let landed, landed != selection else { return }
      onShow(landed)
    }
  }
}

private struct PagedTabs: ViewModifier {
  func body(content: Content) -> some View {
    #if os(iOS)
      content.tabViewStyle(.page(indexDisplayMode: .never))
    #else
      content
    #endif
  }
}
