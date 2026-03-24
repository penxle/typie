package co.typie.editor

import co.typie.di.PlatformContext
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single

@Module
expect class EditorModule() {
    @Single fun editorEngine(ctx: PlatformContext): EditorEngine
}

class EditorException(message: String) : RuntimeException(message)

data class PageRenderInfo(
    val width: Int,
    val height: Int,
    val bufferSize: Long,
)

data class CharacterCounts(
    val docWithWhitespace: Int,
    val docWithoutWhitespace: Int,
    val docWithoutWhitespaceAndPunctuation: Int,
    val selectionWithWhitespace: Int,
    val selectionWithoutWhitespace: Int,
    val selectionWithoutWhitespaceAndPunctuation: Int,
)

data class DragImageData(
    val width: Int,
    val height: Int,
    val offsetX: Float,
    val offsetY: Float,
    val scaleFactor: Float,
    val pixels: ByteArray,
)

interface EditorEngine {
    fun validateRegex(pattern: String): Boolean
    fun createEditor(scaleFactor: Double, snapshot: ByteArray? = null): Editor
    fun addFontBase(family: String, weight: Int, data: ByteArray)
    fun addFontChunk(family: String, weight: Int, data: ByteArray)
    fun setAvailableFonts(fontsJson: String)
    fun setTextReplacementRules(rulesJson: String)
    fun getSlateOffsets(): String
    fun close()
}

interface Editor {
    fun dispatch(messageJson: String)
    fun tick()
    fun flush()

    fun getPageCount(): Int
    fun getRenderInfo(pageIndex: Int): PageRenderInfo?
    fun renderDragImage(visiblePages: List<Int>, pageIdx: Int): DragImageData?

    fun isSelectionHit(pageIdx: Int, x: Float, y: Float): Boolean
    fun isCursorHit(pageIdx: Int, x: Float, y: Float): Boolean

    fun export(mode: Int, version: ByteArray? = null): ByteArray
    fun importUpdates(data: ByteArray)
    fun importUpdatesBatch(updates: List<ByteArray>)

    fun getCharacterCounts(): CharacterCounts
    fun getClipboardData(): String?
    fun getTextWithMappings(): String

    fun performSearch(query: String, matchWholeWord: Boolean): String
    fun setTrackedItems(group: Int, itemsJson: String)
    fun removeTrackedItems(group: Int, idsJson: String)
    fun revealTrackedItem(group: Int, id: String): Boolean

    fun replaceTextInBlock(blockId: String, startOffset: Int, endOffset: Int, replacement: String): Boolean
    fun replaceTextInBlocks(itemsJson: String)
    fun insertTemplateFragment(snapshot: ByteArray)

    fun setTracing(traceId: String, parentSpanId: String)
    fun clearTracing()
    fun drainTraces(): String

    fun close()
}
