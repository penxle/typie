#if canImport(UIKit)

  import Core
  import Design
  import UIKit

  final class HeroHostController: UIViewController {
    private let titleState: HeroTitleState
    private let content: UIViewController
    private let titleLabel = UILabel()

    init(title: HeroTitleState, content: UIViewController) {
      titleState = title
      self.content = content
      super.init(nibName: nil, bundle: nil)
      navigationItem.backButtonDisplayMode = .minimal
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func viewDidLoad() {
      super.viewDidLoad()
      view.backgroundColor = .clear
      embed(content)
      titleLabel.text = title
      navigationItem.title = nil
      titleLabel.font = TTypography.title.uiFont(for: traitCollection)
      titleLabel.textColor = .theme(\.textDefault)
      titleLabel.alpha = 0
      titleLabel.sizeToFit()
      let titleContainer = UIView(frame: titleLabel.bounds)
      titleContainer.addSubview(titleLabel)
      navigationItem.titleView = titleContainer
      registerForTraitChanges([UITraitPreferredContentSizeCategory.self]) {
        (self: HeroHostController, _) in
        self.layoutTitle()
      }
      keepObserving(while: self) { [weak self] in
        guard let self else { return }
        let visible = titleState.titleVisible
        let alpha: CGFloat = visible ? 1 : 0
        guard titleLabel.alpha != alpha else { return }
        let duration = UIAccessibility.isReduceMotionEnabled ? 0 : 0.2
        UIView.animate(
          withDuration: duration, delay: 0, options: [.curveEaseOut, .beginFromCurrentState]
        ) {
          self.titleLabel.alpha = alpha
        }
      }
    }

    private func layoutTitle() {
      titleLabel.font = TTypography.title.uiFont(for: traitCollection)
      titleLabel.sizeToFit()
      navigationItem.titleView?.frame = titleLabel.bounds
      navigationController?.navigationBar.setNeedsLayout()
    }
  }

#endif
