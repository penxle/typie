#if canImport(UIKit)

  import Design
  import SwiftUI

  @MainActor
  struct HomeCustomizeScreen: View {
    static let rowHeight: CGFloat = 56
    private static let separator: CGFloat = 1
    private static let pitch = rowHeight + separator
    private static let introHeight: CGFloat = 22 + 2 + 20 + 8
    private static let settle = Animation.spring(duration: 0.3, bounce: 0)
    static let contentHeight: CGFloat = {
      let count = CGFloat(HomeSection.allCases.count)
      return 8 + introHeight + rowHeight * count + separator * (count - 1) + 8
    }()

    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }
    @State private var drag: Drag?
    @State private var moves = 0
    @State private var lifts = 0
    @State private var order: [HomeSection]

    private let layout: HomeLayoutStore

    init(layout: HomeLayoutStore) {
      self.layout = layout
      _order = State(initialValue: layout.order)
    }

    private struct Drag {
      let section: HomeSection
      let startIndex: Int
      var translation: CGFloat
    }

    var body: some View {
      ScrollView {
        VStack(alignment: .leading, spacing: 0) {
          TText("홈 사용자 정의", style: TTypography.title, color: colors.textDefault)
          TText(
            "섹션을 숨기거나 재정렬해 홈을 맞춤 설정하세요.", style: TTypography.detail,
            color: colors.textMuted
          )
          .padding(.top, 2)
          card
            .padding(.top, 8)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .frame(maxWidth: 600)
        .frame(maxWidth: .infinity, alignment: .leading)
      }
      .scrollDisabled(true)
      .sheetBackground()
    }

    private var card: some View {
      VStack(spacing: 0) {
        ForEach(Array(order.enumerated()), id: \.element.id) { index, section in
          let dragging = drag?.section == section
          row(section)
            .scaleEffect(dragging && !reduceMotion ? 1.02 : 1)
            .animation(.easeOut(duration: 0.15), value: dragging)
            .offset(y: offset(for: section))
            .zIndex(dragging ? 1 : 0)
            .animation(
              drag?.section == section || reduceMotion ? nil : Self.settle, value: order)
          if index < order.count - 1 {
            Rectangle()
              .fill(colors.borderHairline)
              .frame(height: Self.separator)
              .zIndex(-1)
          }
        }
      }
      .coordinateSpace(name: "card")
      .padding(.horizontal, 10)
      .background(
        colors.textDefault.opacity(SiteSwitcherScreen.cardInsetOpacity),
        in: TShapes.squircle(TShapes.lg)
      )
      .sensoryFeedback(.selection, trigger: moves)
      .sensoryFeedback(.impact(weight: .light), trigger: lifts)
    }

    private func row(_ section: HomeSection) -> some View {
      HStack(spacing: 0) {
        if section.canHide {
          let visible = layout.isVisible(section)
          Button {
            layout.setVisible(section, !visible)
          } label: {
            label(section, check: CheckCircle(on: visible))
          }
          .buttonStyle(.plain)
          .accessibilityLabel(section.title)
          .accessibilityValue(visible ? "표시" : "숨김")
        } else {
          label(section, check: Color.clear)
        }
        if section == .recent {
          let sort = layout.recentSort
          Button {
            layout.setRecentSort(sort.next)
          } label: {
            HStack(spacing: 4) {
              TIcon(
                LucideIcon.arrowLeftRight, size: 14, tint: colors.textHint,
                relativeTo: TTypography.detail)
              TText(sort.shortTitle, style: TTypography.detail, color: colors.textHint)
                .id(sort)
                .transition(.opacity)
            }
            .padding(.horizontal, 8)
            .frame(minHeight: Self.rowHeight)
            .contentShape(Rectangle())
            .animation(reduceMotion ? nil : .easeOut(duration: 0.12), value: sort)
          }
          .buttonStyle(.plain)
          .accessibilityLabel("최근 문서 정렬")
          .accessibilityValue(sort.title)
        }
        if section.canHide {
          Button {
            layout.setVisible(section, !layout.isVisible(section))
          } label: {
            Color.clear
              .frame(maxWidth: .infinity, minHeight: Self.rowHeight)
              .contentShape(Rectangle())
          }
          .buttonStyle(.plain)
          .accessibilityHidden(true)
        } else {
          Spacer(minLength: 0)
        }
        TIcon(LucideIcon.gripVertical, size: 18, tint: colors.textHint)
          .frame(width: 44, height: Self.rowHeight, alignment: .trailing)
          .contentShape(Rectangle())
          .gesture(handleGesture(section))
          .accessibilityHidden(true)
      }
      .frame(height: Self.rowHeight)
      .accessibilityElement(children: .contain)
      .accessibilityAction(named: "위로 이동") { move(section, by: -1) }
      .accessibilityAction(named: "아래로 이동") { move(section, by: 1) }
    }

    private func label(_ section: HomeSection, check: some View) -> some View {
      HStack(spacing: 12) {
        check.frame(width: CheckCircle.side, height: CheckCircle.side)
        TText(section.title, style: TTypography.text, color: colors.textDefault)
      }
      .frame(minHeight: Self.rowHeight)
      .contentShape(Rectangle())
    }

    private func offset(for section: HomeSection) -> CGFloat {
      guard let drag, drag.section == section, let index = index(of: section) else { return 0 }
      return drag.translation - CGFloat(index - drag.startIndex) * Self.pitch
    }

    private func index(of section: HomeSection) -> Int? {
      order.firstIndex(of: section)
    }

    private func handleGesture(_ section: HomeSection) -> some Gesture {
      DragGesture(minimumDistance: 0, coordinateSpace: .named("card"))
        .onChanged { value in
          guard let current = index(of: section) else { return }
          if drag == nil {
            drag = Drag(section: section, startIndex: current, translation: 0)
            lifts += 1
          }
          if let startIndex = drag?.startIndex {
            let lower = -CGFloat(startIndex) * Self.pitch
            let upper = CGFloat(order.count - 1 - startIndex) * Self.pitch
            drag?.translation = min(max(value.translation.height, lower), upper)
          }
          guard let drag else { return }
          let steps = Int((drag.translation / Self.pitch).rounded())
          let target = min(max(drag.startIndex + steps, 0), order.count - 1)
          if target != current {
            order.move(
              fromOffsets: IndexSet(integer: current),
              toOffset: target > current ? target + 1 : target)
            moves += 1
          }
        }
        .onEnded { _ in
          withAnimation(reduceMotion ? nil : Self.settle) { drag = nil }
          layout.setOrder(order)
        }
    }

    private func move(_ section: HomeSection, by delta: Int) {
      guard let current = index(of: section) else { return }
      let target = current + delta
      guard order.indices.contains(target) else { return }
      withAnimation(reduceMotion ? nil : Self.settle) {
        order.move(
          fromOffsets: IndexSet(integer: current), toOffset: delta > 0 ? target + 1 : target)
      }
      layout.setOrder(order)
    }
  }

  private struct CheckCircle: View {
    static let side: CGFloat = 22

    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    private var colors: TColors { theme.colors }

    let on: Bool

    var body: some View {
      ZStack {
        Circle()
          .strokeBorder(colors.borderEmphasis, lineWidth: 1.5)
          .opacity(on ? 0 : 1)
        Circle()
          .fill(colors.accentDefault)
          .scaleEffect(on || reduceMotion ? 1 : 0.6)
          .opacity(on ? 1 : 0)
        TIcon(LucideIcon.check, size: 12, tint: colors.surfaceCanvas)
          .opacity(on ? 1 : 0)
      }
      .frame(width: Self.side, height: Self.side)
      .contentShape(Rectangle())
      .animation(.easeOut(duration: 0.12), value: on)
    }
  }

#endif
