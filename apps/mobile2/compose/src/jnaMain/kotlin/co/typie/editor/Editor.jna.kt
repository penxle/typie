package co.typie.editor

import uniffi.editor.EditorEngine as NativeEditorEngine
import uniffi.editor.Editor as NativeEditor
import uniffi.editor.EditorException as NativeEditorException
import uniffi.editor.PageRenderInfo as NativePageRenderInfo
import uniffi.editor.PageTexture as NativePageTexture
import uniffi.editor.CharacterCounts as NativeCharacterCounts
import uniffi.editor.DragImageData as NativeDragImageData

private inline fun <T> wrapCall(block: () -> T): T {
    try {
        return block()
    } catch (e: NativeEditorException) {
        throw EditorException(e.message ?: "Unknown editor error")
    }
}

class JnaEditorEngine(private val native: NativeEditorEngine) : EditorEngine {
    override suspend fun initGpu(): Boolean {
        return native.initGpu()
    }

    override fun validateRegex(pattern: String): Boolean {
        return wrapCall { native.validateRegex(pattern) }
    }

    override fun createEditor(scaleFactor: Double, snapshot: ByteArray?): Editor {
        return wrapCall { JnaEditor(native.createEditor(scaleFactor, snapshot)) }
    }

    override fun addFontBase(family: String, weight: Int, data: ByteArray) {
        wrapCall { native.addFontBase(family, weight.toUShort(), data) }
    }

    override fun addFontChunk(family: String, weight: Int, data: ByteArray) {
        wrapCall { native.addFontChunk(family, weight.toUShort(), data) }
    }

    override fun setAvailableFonts(fontsJson: String) {
        wrapCall { native.setAvailableFonts(fontsJson) }
    }

    override fun setTextReplacementRules(rulesJson: String) {
        wrapCall { native.setTextReplacementRules(rulesJson) }
    }

    override fun getSlateOffsets(): String {
        return wrapCall { native.getSlateOffsets() }
    }

    override fun close() {
        native.close()
    }
}

class JnaEditor(private val native: NativeEditor) : Editor {
    override fun dispatch(messageJson: String) {
        wrapCall { native.dispatch(messageJson) }
    }

    override fun tick() {
        wrapCall { native.tick() }
    }

    override fun flush() {
        wrapCall { native.flush() }
    }

    override fun attachSurface(pageIndex: Int): PageTexture {
        return wrapCall { JnaPageTexture(native.attachSurface(pageIndex.toUInt())) }
    }

    override fun detachSurface(pageIndex: Int) {
        native.detachSurface(pageIndex.toUInt())
    }

    override fun getPageCount(): Int {
        return wrapCall { native.getPageCount().toInt() }
    }

    override fun getRenderInfo(pageIndex: Int): PageRenderInfo? {
        return wrapCall { native.getRenderInfo(pageIndex.toUInt())?.toKotlin() }
    }

    override fun renderDragImage(visiblePages: List<Int>, pageIdx: Int): DragImageData? {
        return wrapCall { native.renderDragImage(visiblePages.map { it.toUInt() }, pageIdx.toUInt())?.toKotlin() }
    }

    override fun isSelectionHit(pageIdx: Int, x: Float, y: Float): Boolean {
        return wrapCall { native.isSelectionHit(pageIdx.toUInt(), x, y) }
    }

    override fun isCursorHit(pageIdx: Int, x: Float, y: Float): Boolean {
        return wrapCall { native.isCursorHit(pageIdx.toUInt(), x, y) }
    }

    override fun export(mode: Int, version: ByteArray?): ByteArray {
        return wrapCall { native.export(mode, version) }
    }

    override fun importUpdates(data: ByteArray) {
        wrapCall { native.importUpdates(data) }
    }

    override fun importUpdatesBatch(updates: List<ByteArray>) {
        wrapCall { native.importUpdatesBatch(updates) }
    }

    override fun getCharacterCounts(): CharacterCounts {
        return wrapCall { native.getCharacterCounts().toKotlin() }
    }

    override fun getClipboardData(): String? {
        return wrapCall { native.getClipboardData() }
    }

    override fun getTextWithMappings(): String {
        return wrapCall { native.getTextWithMappings() }
    }

    override fun performSearch(query: String, matchWholeWord: Boolean): String {
        return wrapCall { native.performSearch(query, matchWholeWord) }
    }

    override fun setTrackedItems(group: Int, itemsJson: String) {
        wrapCall { native.setTrackedItems(group.toUInt(), itemsJson) }
    }

    override fun removeTrackedItems(group: Int, idsJson: String) {
        wrapCall { native.removeTrackedItems(group.toUInt(), idsJson) }
    }

    override fun revealTrackedItem(group: Int, id: String): Boolean {
        return wrapCall { native.revealTrackedItem(group.toUInt(), id) }
    }

    override fun replaceTextInBlock(blockId: String, startOffset: Int, endOffset: Int, replacement: String): Boolean {
        return wrapCall { native.replaceTextInBlock(blockId, startOffset.toUInt(), endOffset.toUInt(), replacement) }
    }

    override fun replaceTextInBlocks(itemsJson: String) {
        wrapCall { native.replaceTextInBlocks(itemsJson) }
    }

    override fun insertTemplateFragment(snapshot: ByteArray) {
        wrapCall { native.insertTemplateFragment(snapshot) }
    }

    override fun setTracing(traceId: String, parentSpanId: String) {
        wrapCall { native.setTracing(traceId, parentSpanId) }
    }

    override fun clearTracing() {
        wrapCall { native.clearTracing() }
    }

    override fun drainTraces(): String {
        return wrapCall { native.drainTraces() }
    }

    override fun close() {
        native.close()
    }
}

class JnaPageTexture(private val native: NativePageTexture) : PageTexture {
    override val nativeHandle: Long = native.nativeHandle().toLong()
    override val width: Int = native.width().toInt()
    override val height: Int = native.height().toInt()
    override fun pixelData(): ByteArray? = native.pixelData()
    override fun close() = native.close()
}

private fun NativePageRenderInfo.toKotlin() = PageRenderInfo(
    width = width.toInt(),
    height = height.toInt(),
    bufferSize = bufferSize.toLong(),
)

private fun NativeCharacterCounts.toKotlin() = CharacterCounts(
    docWithWhitespace = docWithWhitespace.toInt(),
    docWithoutWhitespace = docWithoutWhitespace.toInt(),
    docWithoutWhitespaceAndPunctuation = docWithoutWhitespaceAndPunctuation.toInt(),
    selectionWithWhitespace = selectionWithWhitespace.toInt(),
    selectionWithoutWhitespace = selectionWithoutWhitespace.toInt(),
    selectionWithoutWhitespaceAndPunctuation = selectionWithoutWhitespaceAndPunctuation.toInt(),
)

private fun NativeDragImageData.toKotlin() = DragImageData(
    width = width.toInt(),
    height = height.toInt(),
    offsetX = offsetX,
    offsetY = offsetY,
    scaleFactor = scaleFactor,
    pixels = pixels,
)
