#if canImport(UIKit)

  import Core
  import Design
  import Editor
  import FactoryKit
  import UIKit

  final class DocumentScreenController: UIViewController {
    private let resources = Container.shared.editorResources()
    private let fonts = Container.shared.fontLoader()
    private let fontFamilies = Container.shared.documentFontFamiliesModel()
    private let toast = Container.shared.toast()
    private var editor: EditorView?
    private var opening: Task<Void, Never>?
    private var layout = EditorSyntheticLayout.paginated

    var debugOverlays: EditorDebugOverlays = [] {
      didSet { editor?.debugOverlays = debugOverlays }
    }

    init() {
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
    }

    override func viewIsAppearing(_ animated: Bool) {
      super.viewIsAppearing(animated)
      guard opening == nil else { return }
      open(layout)
    }

    func switchSyntheticLayout() {
      layout = layout == .paginated ? .continuous : .paginated
      open(layout)
    }

    private func open(_ layout: EditorSyntheticLayout) {
      let previous = opening
      opening = Task { [weak self] in
        await previous?.value
        guard let self else { return }
        do {
          try await editorView().open(.synthetic(layout))
        } catch {
          showError()
        }
      }
    }

    private func editorView() async throws -> EditorView {
      if let editor { return editor }
      let resources = try await resources.value
      let fonts = try await fonts.value
      let editor = EditorView(resources: resources, fonts: fonts)
      editor.frame = view.bounds
      editor.autoresizingMask = [.flexibleWidth, .flexibleHeight]
      editor.debugOverlays = debugOverlays
      view.addSubview(editor)
      self.editor = editor
      keepObserving(while: self) { [weak self] in
        guard let self else { return }
        if fontFamilies.loadFailed {
          showError()
        }
        guard let families = fontFamilies.families else { return }
        do {
          try fonts.apply(families)
        } catch {
          showError()
        }
      }
      return editor
    }

    private func showError() {
      toast.error("오류가 발생했어요. 잠시 후 다시 시도해주세요.")
    }
  }

#endif
