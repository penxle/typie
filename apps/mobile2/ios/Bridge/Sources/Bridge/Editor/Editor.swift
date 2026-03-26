import Foundation

@objc public class NativePageRenderInfo: NSObject {
    @objc public let width: Int32
    @objc public let height: Int32
    @objc public let bufferSize: Int64

    init(_ info: PageRenderInfo) {
        self.width = Int32(info.width)
        self.height = Int32(info.height)
        self.bufferSize = Int64(info.bufferSize)
    }
}

@objc public class NativeCharacterCounts: NSObject {
    @objc public let docWithWhitespace: Int32
    @objc public let docWithoutWhitespace: Int32
    @objc public let docWithoutWhitespaceAndPunctuation: Int32
    @objc public let selectionWithWhitespace: Int32
    @objc public let selectionWithoutWhitespace: Int32
    @objc public let selectionWithoutWhitespaceAndPunctuation: Int32

    init(_ counts: CharacterCounts) {
        self.docWithWhitespace = Int32(counts.docWithWhitespace)
        self.docWithoutWhitespace = Int32(counts.docWithoutWhitespace)
        self.docWithoutWhitespaceAndPunctuation = Int32(counts.docWithoutWhitespaceAndPunctuation)
        self.selectionWithWhitespace = Int32(counts.selectionWithWhitespace)
        self.selectionWithoutWhitespace = Int32(counts.selectionWithoutWhitespace)
        self.selectionWithoutWhitespaceAndPunctuation = Int32(counts.selectionWithoutWhitespaceAndPunctuation)
    }
}

@objc public class NativeDragImageData: NSObject {
    @objc public let width: Int32
    @objc public let height: Int32
    @objc public let offsetX: Float
    @objc public let offsetY: Float
    @objc public let scaleFactor: Float
    @objc public let pixels: Data

    init(_ data: DragImageData) {
        self.width = Int32(data.width)
        self.height = Int32(data.height)
        self.offsetX = data.offsetX
        self.offsetY = data.offsetY
        self.scaleFactor = data.scaleFactor
        self.pixels = data.pixels
    }
}

@objc public class NativePageTexture: NSObject {
    private let texture: PageTexture

    @objc public let nativeHandle: Int64
    @objc public let width: Int32
    @objc public let height: Int32

    init(_ texture: PageTexture) {
        self.texture = texture
        self.nativeHandle = Int64(texture.nativeHandle())
        self.width = Int32(texture.width())
        self.height = Int32(texture.height())
    }

    @objc public func pixelData() -> Data? {
        return texture.pixelData()
    }

    @objc public func close() {
        // UniFFI PageTexture는 deinit에서 자동 정리됨
    }
}

@objc public class NativeOptionalPageRenderInfo: NSObject {
    @objc public let value: NativePageRenderInfo?

    init(_ info: PageRenderInfo?) {
        self.value = info.map { NativePageRenderInfo($0) }
    }
}

@objc public class NativeOptionalDragImageData: NSObject {
    @objc public let value: NativeDragImageData?

    init(_ data: DragImageData?) {
        self.value = data.map { NativeDragImageData($0) }
    }
}

@objc public class NativeOptionalString: NSObject {
    @objc public let value: String?

    init(_ s: String?) {
        self.value = s
    }
}

@objc public class NativeEditorEngine: NSObject {
    private let engine: EditorEngine

    @objc public override init() {
        self.engine = EditorEngine()
        super.init()
    }

    @objc public func loadIcuData(_ data: Data) throws {
        try engine.loadIcuData(data: data)
    }

    @objc public func validateRegex(pattern: String) throws -> NSNumber {
        return NSNumber(value: engine.validateRegex(pattern: pattern))
    }

    @objc public func createEditor(scaleFactor: Double, snapshot: Data?) throws -> NativeEditor {
        let editor = try engine.createEditor(scaleFactor: scaleFactor, snapshot: snapshot)
        return NativeEditor(editor: editor)
    }

    @objc public func addFontBase(family: String, weight: Int32, data: Data) throws {
        try engine.addFontBase(family: family, weight: UInt16(weight), data: data)
    }

    @objc public func addFontChunk(family: String, weight: Int32, data: Data) throws {
        try engine.addFontChunk(family: family, weight: UInt16(weight), data: data)
    }

    @objc public func setAvailableFonts(fontsJson: String) throws {
        try engine.setAvailableFonts(fontsJson: fontsJson)
    }

    @objc public func setTextReplacementRules(rulesJson: String) throws {
        try engine.setTextReplacementRules(rulesJson: rulesJson)
    }

    @objc public func getSlateOffsets() throws -> String {
        return try engine.getSlateOffsets()
    }
}

@objc public class NativeEditor: NSObject {
    private let editor: Editor

    init(editor: Editor) {
        self.editor = editor
        super.init()
    }

    @objc public func attachSurface(pageIndex: Int32) throws -> NativePageTexture {
        return NativePageTexture(try editor.attachSurface(pageIndex: UInt32(pageIndex)))
    }

    @objc public func detachSurface(pageIndex: Int32) throws {
        try editor.detachSurface(pageIndex: UInt32(pageIndex))
    }

    @objc public func dispatch(messageJson: String) throws {
        try editor.dispatch(messageJson: messageJson)
    }

    @objc public func tick() throws {
        try editor.tick()
    }

    @objc public func flush() throws {
        try editor.flush()
    }

    @objc public func getPageCount() throws -> NSNumber {
        return NSNumber(value: try editor.getPageCount())
    }

    @objc public func getRenderInfo(pageIndex: Int32) throws -> NativeOptionalPageRenderInfo {
        let info = try editor.getRenderInfo(pageIndex: UInt32(pageIndex))
        return NativeOptionalPageRenderInfo(info)
    }

    @objc public func renderDragImage(visiblePages: [NSNumber], pageIdx: Int32) throws -> NativeOptionalDragImageData {
        let pages = visiblePages.map { UInt32(truncating: $0) }
        let result = try editor.renderDragImage(visiblePages: pages, pageIdx: UInt32(pageIdx))
        return NativeOptionalDragImageData(result)
    }

    @objc public func isSelectionHit(pageIdx: Int32, x: Float, y: Float) throws -> NSNumber {
        return NSNumber(value: try editor.isSelectionHit(pageIdx: UInt32(pageIdx), x: x, y: y))
    }

    @objc public func isCursorHit(pageIdx: Int32, x: Float, y: Float) throws -> NSNumber {
        return NSNumber(value: try editor.isCursorHit(pageIdx: UInt32(pageIdx), x: x, y: y))
    }

    @objc public func export(mode: Int32, version: Data?) throws -> Data {
        return try editor.export(mode: mode, version: version)
    }

    @objc public func importUpdates(_ data: Data) throws {
        try editor.importUpdates(data: data)
    }

    @objc public func importUpdatesBatch(_ updates: [Data]) throws {
        try editor.importUpdatesBatch(updates: updates)
    }

    @objc public func getCharacterCounts() throws -> NativeCharacterCounts {
        return NativeCharacterCounts(try editor.getCharacterCounts())
    }

    @objc public func getClipboardData() throws -> NativeOptionalString {
        let result = try editor.getClipboardData()
        return NativeOptionalString(result)
    }

    @objc public func getTextWithMappings() throws -> String {
        return try editor.getTextWithMappings()
    }

    @objc public func performSearch(query: String, matchWholeWord: Bool) throws -> String {
        return try editor.performSearch(query: query, matchWholeWord: matchWholeWord)
    }

    @objc public func setTrackedItems(group: Int32, itemsJson: String) throws {
        try editor.setTrackedItems(group: UInt32(group), itemsJson: itemsJson)
    }

    @objc public func removeTrackedItems(group: Int32, idsJson: String) throws {
        try editor.removeTrackedItems(group: UInt32(group), idsJson: idsJson)
    }

    @objc public func revealTrackedItem(group: Int32, id: String) throws -> NSNumber {
        return NSNumber(value: try editor.revealTrackedItem(group: UInt32(group), id: id))
    }

    @objc public func replaceTextInBlock(blockId: String, startOffset: Int32, endOffset: Int32, replacement: String) throws -> NSNumber {
        return NSNumber(value: try editor.replaceTextInBlock(blockId: blockId, startOffset: UInt32(startOffset), endOffset: UInt32(endOffset), replacement: replacement))
    }

    @objc public func replaceTextInBlocks(itemsJson: String) throws {
        try editor.replaceTextInBlocks(itemsJson: itemsJson)
    }

    @objc public func insertTemplateFragment(_ snapshot: Data) throws {
        try editor.insertTemplateFragment(snapshot: snapshot)
    }

    @objc public func setTracing(traceId: String, parentSpanId: String) throws {
        try editor.setTracing(traceId: traceId, parentSpanId: parentSpanId)
    }

    @objc public func clearTracing() throws {
        try editor.clearTracing()
    }

    @objc public func drainTraces() throws -> String {
        return try editor.drainTraces()
    }
}
