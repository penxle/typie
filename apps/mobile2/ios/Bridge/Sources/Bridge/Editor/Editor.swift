import Foundation

@objc public class NativeEditorEngine: NSObject {
    private let engine: EditorEngine

    @objc public override init() {
        self.engine = EditorEngine()
        super.init()
    }

    @objc public func loadIcuData(_ data: Data) throws {
        try engine.loadIcuData(data: data)
    }

    @objc public func createEditor(scaleFactor: Double, snapshot: Data?) throws -> NativeEditor {
        let editor = try engine.createEditor(scaleFactor: scaleFactor, snapshot: snapshot)
        return NativeEditor(editor: editor)
    }
}

@objc public class NativeEditor: NSObject {
    private let editor: Editor

    init(editor: Editor) {
        self.editor = editor
        super.init()
    }

    @objc public func dispatch(messageJson: String) throws {
        try editor.dispatch(messageJson: messageJson)
    }

    @objc public func tick() throws {
        try editor.tick()
    }

    @objc public func exportSnapshot() throws -> Data {
        return try editor.exportSnapshot()
    }
}
