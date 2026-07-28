import Compose
import UIKit

final class ShortcutHostingViewController: UIViewController {
  private let content: UIViewController
  private let shortcutRegistry: NativeShortcutRegistry

  init(content: UIViewController, shortcutRegistry: NativeShortcutRegistry) {
    self.content = content
    self.shortcutRegistry = shortcutRegistry
    super.init(nibName: nil, bundle: nil)
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  override func viewDidLoad() {
    super.viewDidLoad()
    addChild(content)
    content.view.frame = view.bounds
    content.view.autoresizingMask = [.flexibleWidth, .flexibleHeight]
    view.addSubview(content.view)
    content.didMove(toParent: self)
  }

  override var childForStatusBarStyle: UIViewController? {
    content
  }

  override var childForStatusBarHidden: UIViewController? {
    content
  }

  override var keyCommands: [UIKeyCommand]? {
    guard let input = shortcutRegistry.activeInput() else {
      return super.keyCommands
    }

    let command = UIKeyCommand(
      input: input,
      modifierFlags: UIKeyModifierFlags(
        rawValue: Int(shortcutRegistry.activeModifierFlags())
      ),
      action: #selector(handleShortcut(_:))
    )
    command.wantsPriorityOverSystemBehavior = true
    return (super.keyCommands ?? []) + [command]
  }

  @objc private func handleShortcut(_ command: UIKeyCommand) {
    shortcutRegistry.dispatch(
      input: command.input ?? "",
      modifierFlags: Int64(command.modifierFlags.rawValue)
    )
  }
}
