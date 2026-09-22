#if canImport(UIKit)

  import UIKit

  final class SearchScreenController: UIViewController {
    private let model: SearchModel
    private let content: UIViewController

    init(
      model: SearchModel, keyboardInset: CGFloat, onOpen: @escaping (SearchHit) -> Void,
      onDismissKeyboard: @escaping () -> Void
    ) {
      self.model = model
      content = ThemedHostingController(
        title: "",
        SearchScreen(
          model: model, keyboardInset: keyboardInset, onOpen: onOpen,
          onDismissKeyboard: onDismissKeyboard))
      super.init(nibName: nil, bundle: nil)
      navigationItem.backButtonDisplayMode = .minimal
      navigationItem.hidesBackButton = true
      let appearance = UINavigationBarAppearance()
      appearance.configureWithTransparentBackground()
      navigationItem.standardAppearance = appearance
      navigationItem.scrollEdgeAppearance = appearance
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
      nil
    }

    override func viewDidLoad() {
      super.viewDidLoad()
      view.backgroundColor = .clear
      embed(content)
    }

    override func viewDidLayoutSubviews() {
      super.viewDidLayoutSubviews()
      guard let bar = navigationController?.navigationBar else { return }
      let inset = -bar.frame.height
      if additionalSafeAreaInsets.top != inset {
        additionalSafeAreaInsets.top = inset
      }
    }

    override func didMove(toParent parent: UIViewController?) {
      super.didMove(toParent: parent)
      if parent == nil { model.setText("") }
    }
  }

#endif
