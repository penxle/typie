#if canImport(UIKit)

  import Design
  import SwiftUI
  import UIKit

  private final class ViewerOffsetEffect: UIMotionEffect {
    var onChange: ((UIOffset) -> Void)?

    override func keyPathsAndRelativeValues(forViewerOffset viewerOffset: UIOffset)
      -> [String: Any]?
    {
      onChange?(viewerOffset)
      return nil
    }
  }

  struct ViewerOffsetReader: UIViewRepresentable {
    let onChange: (UIOffset) -> Void

    func makeUIView(context: Context) -> UIView {
      let view = UIView()
      view.isUserInteractionEnabled = false
      let effect = ViewerOffsetEffect()
      effect.onChange = onChange
      view.addMotionEffect(effect)
      return view
    }

    func updateUIView(_ uiView: UIView, context: Context) {
      (uiView.motionEffects.first as? ViewerOffsetEffect)?.onChange = onChange
    }
  }

  struct LoginBackdrop: View {
    @Environment(\.theme) private var theme
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.colorScheme) private var colorScheme
    private var colors: TColors { theme.colors }

    private func glow(_ color: Color, opacity: Double) -> Color {
      colorScheme == .dark
        ? color.mix(with: .black, by: 0.4).opacity(opacity * 1.8)
        : color.opacity(opacity)
    }

    private struct Blob {
      let color: KeyPath<TColors, Color>
      let opacity: Double
      let size: CGFloat
      let origin: CGPoint
      let sway: CGSize
      let period: Double
      let phase: Double
    }

    private let blobs = [
      Blob(
        color: \.paletteBlue, opacity: 0.22, size: 520, origin: CGPoint(x: -160, y: -80),
        sway: CGSize(width: 120, height: 110), period: 14, phase: 0),
      Blob(
        color: \.paletteRed, opacity: 0.16, size: 520, origin: CGPoint(x: 180, y: 300),
        sway: CGSize(width: 110, height: 140), period: 17, phase: 2.1),
      Blob(
        color: \.paletteYellow, opacity: 0.16, size: 460, origin: CGPoint(x: -60, y: 560),
        sway: CGSize(width: 140, height: 110), period: 20, phase: 4.2),
    ]

    private func position(of blob: Blob, at seconds: Double) -> CGPoint {
      let angle = seconds / blob.period * 2 * .pi + blob.phase
      return CGPoint(
        x: blob.origin.x + blob.sway.width * CGFloat(sin(angle)),
        y: blob.origin.y + blob.sway.height * CGFloat(cos(angle * 0.8)))
    }

    var body: some View {
      let glows = blobs.map { glow(colors[keyPath: $0.color], opacity: $0.opacity) }
      TimelineView(.animation(minimumInterval: 1 / 30, paused: reduceMotion)) { context in
        let seconds = context.date.timeIntervalSinceReferenceDate
        ZStack(alignment: .topLeading) {
          colors.surfaceCanvas
          ForEach(Array(blobs.enumerated()), id: \.offset) { index, blob in
            let point = position(of: blob, at: reduceMotion ? 0 : seconds)
            Circle()
              .fill(glows[index])
              .frame(width: blob.size, height: blob.size)
              .blur(radius: 84)
              .offset(x: point.x, y: point.y)
          }
        }
      }
      .clipped()
      .ignoresSafeArea()
      .allowsHitTesting(false)
    }
  }

#endif
