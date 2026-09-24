#if canImport(UIKit)

  import Design
  import UIKit

  final class EditorPositionPill: UIView {
    private static let horizontalPadding: CGFloat = 12
    private static let verticalPadding: CGFloat = 6

    private let material = UIVisualEffectView(effect: nil)
    private let label = UILabel()
    private(set) var isShown = false

    var text: String? {
      get { label.text }
      set { label.text = newValue }
    }

    override init(frame: CGRect) {
      super.init(frame: frame)
      isUserInteractionEnabled = false
      isHidden = true
      material.translatesAutoresizingMaskIntoConstraints = false
      label.translatesAutoresizingMaskIntoConstraints = false
      label.font = Self.font(for: traitCollection)
      label.textColor = .theme(\.textDefault)
      label.alpha = 0
      addSubview(material)
      material.contentView.addSubview(label)
      NSLayoutConstraint.activate([
        material.leadingAnchor.constraint(equalTo: leadingAnchor),
        material.trailingAnchor.constraint(equalTo: trailingAnchor),
        material.topAnchor.constraint(equalTo: topAnchor),
        material.bottomAnchor.constraint(equalTo: bottomAnchor),
        label.leadingAnchor.constraint(
          equalTo: material.contentView.leadingAnchor, constant: Self.horizontalPadding),
        label.trailingAnchor.constraint(
          equalTo: material.contentView.trailingAnchor, constant: -Self.horizontalPadding),
        label.topAnchor.constraint(
          equalTo: material.contentView.topAnchor, constant: Self.verticalPadding),
        label.bottomAnchor.constraint(
          equalTo: material.contentView.bottomAnchor, constant: -Self.verticalPadding),
      ])
      if #available(iOS 26, *) {
        material.cornerConfiguration = .capsule()
      } else {
        material.clipsToBounds = true
      }
      registerForTraitChanges([UITraitPreferredContentSizeCategory.self]) {
        (self: EditorPositionPill, _) in
        self.label.font = Self.font(for: self.traitCollection)
      }
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func layoutSubviews() {
      super.layoutSubviews()
      if #unavailable(iOS 26) {
        material.layer.cornerRadius = bounds.height / 2
      }
    }

    func setShown(_ shown: Bool, duration: TimeInterval) {
      let duration = UIAccessibility.isReduceMotionEnabled ? 0 : duration
      guard shown != isShown || (duration == 0 && !shown && !isHidden) else { return }
      isShown = shown
      isHidden = false
      guard duration > 0 else {
        UIView.performWithoutAnimation {
          material.effect = shown ? Self.effect() : nil
          label.alpha = shown ? 1 : 0
        }
        isHidden = !shown
        return
      }
      UIView.animate(
        withDuration: duration, delay: 0, options: [.curveLinear, .beginFromCurrentState],
        animations: {
          self.material.effect = shown ? Self.effect() : nil
          self.label.alpha = shown ? 1 : 0
        },
        completion: { finished in
          guard finished, !shown, !self.isShown else { return }
          self.isHidden = true
        })
    }

    private static func effect() -> UIVisualEffect {
      if #available(iOS 26, *) {
        return UIGlassEffect()
      }
      return UIBlurEffect(style: .systemUltraThinMaterial)
    }

    private static func font(for traits: UITraitCollection) -> UIFont {
      let font = TTypography.meta.uiFont(for: traits)
      return UIFont(
        descriptor: font.fontDescriptor.addingAttributes([
          .featureSettings: [
            [
              UIFontDescriptor.FeatureKey.type: kNumberSpacingType,
              UIFontDescriptor.FeatureKey.selector: kMonospacedNumbersSelector,
            ]
          ]
        ]), size: 0)
    }
  }

#endif
