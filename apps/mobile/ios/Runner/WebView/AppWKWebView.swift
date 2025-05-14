import WebKit

class AppWKWebView: WKWebView {
  @available(*, unavailable)
  required init?(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  override init(frame: CGRect, configuration: WKWebViewConfiguration) {
    super.init(frame: frame, configuration: configuration)
  }

  override var inputAccessoryView: UIView? {
    return nil
  }
}
