#if canImport(UIKit)

  import UIKit

  @available(iOS 26, *)
  final class MainTabBarBackdrop: UIView {
    private static let edgeSoftness: CGFloat = 18
    private static let reach = edgeSoftness * 3.6
    static let height = reach + MainTabBar.contentBottomInset
    private static let blurRadius: CGFloat = 1
    private static let blurMaskGain: CGFloat = 1.25
    private static let backdropScale: CGFloat = 0.5
    private static let fadeOpacity: CGFloat = 0.85

    private let blurView = BackdropView()
    private let fadeView = UIView()
    private let fadeMask = CALayer()
    private(set) var isConcealed = false

    init(fadeColor: UIColor) {
      super.init(frame: .zero)
      isUserInteractionEnabled = false
      if let blur = ContentBlur.makeFilter(type: "variableBlur") {
        let mask = Self.makeMask { coverage in
          min(max(Self.blurMaskGain * coverage + 1 - Self.blurMaskGain, 0), 1)
        }
        blur.setValue(Self.blurRadius, forKey: "inputRadius")
        blur.setValue(mask, forKey: "inputMaskImage")
        blur.setValue(true, forKey: "inputNormalizeEdges")
        blurView.layer.filters = [blur]
      }
      blurView.layer.setValue(Self.backdropScale, forKey: "scale")
      fadeView.backgroundColor = fadeColor
      fadeView.alpha = Self.fadeOpacity
      fadeMask.contents = Self.makeMask { $0 }
      fadeView.layer.mask = fadeMask
      addSubview(blurView)
      addSubview(fadeView)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func layoutSubviews() {
      super.layoutSubviews()
      blurView.frame = bounds
      fadeView.frame = bounds
      CATransaction.begin()
      CATransaction.setDisableActions(true)
      fadeMask.frame = bounds
      CATransaction.commit()
    }

    func setConcealed(_ concealed: Bool, duration: TimeInterval) {
      guard concealed != isConcealed else { return }
      isConcealed = concealed
      if !concealed { isHidden = false }
      let blurAlpha: CGFloat = concealed ? 0 : 1
      let fadeAlpha = concealed ? 0 : Self.fadeOpacity
      let change = {
        self.blurView.alpha = blurAlpha
        self.fadeView.alpha = fadeAlpha
      }
      guard duration > 0 else {
        change()
        isHidden = concealed
        return
      }
      UIView.animate(
        springDuration: duration, bounce: 0, options: [.beginFromCurrentState],
        animations: change
      ) { _ in
        if self.isConcealed { self.isHidden = true }
      }
    }

    private static func makeMask(_ strength: (CGFloat) -> CGFloat) -> CGImage? {
      let length = Int(height.rounded(.up))
      guard
        let context = CGContext(
          data: nil, width: 1, height: length, bitsPerComponent: 8, bytesPerRow: 0,
          space: CGColorSpaceCreateDeviceGray(),
          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)
      else { return nil }
      for row in 0..<length {
        let alpha = strength(coverage(atHeight: CGFloat(row) + 0.5))
        context.setFillColor(gray: 0, alpha: alpha)
        context.fill(CGRect(x: 0, y: row, width: 1, height: 1))
      }
      return context.makeImage()
    }

    private static func coverage(atHeight y: CGFloat) -> CGFloat {
      let distance = Double(y - MainTabBar.contentBottomInset)
      return CGFloat(0.5 * erfc(distance / (Double(edgeSoftness) * 2.squareRoot())))
    }
  }

  private final class BackdropView: UIView {
    override class var layerClass: AnyClass {
      NSClassFromString("CABackdropLayer") ?? CALayer.self
    }
  }

#endif
