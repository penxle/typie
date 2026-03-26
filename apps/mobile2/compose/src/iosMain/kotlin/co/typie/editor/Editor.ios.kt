package co.typie.editor

import co.typie.di.PlatformContext
import kotlinx.cinterop.ObjCObjectVar
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.alloc
import kotlinx.cinterop.allocArrayOf
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.usePinned
import kotlinx.cinterop.value
import org.koin.core.annotation.Module
import org.koin.core.annotation.Single
import platform.Foundation.NSBundle
import platform.Foundation.NSData
import platform.Foundation.NSError
import platform.Foundation.NSNumber
import platform.Foundation.create
import platform.posix.memcpy
import swiftPMImport.co.typie.compose.NativeCharacterCounts
import swiftPMImport.co.typie.compose.NativeDragImageData
import swiftPMImport.co.typie.compose.NativeEditor
import swiftPMImport.co.typie.compose.NativeEditorEngine
import swiftPMImport.co.typie.compose.NativeOptionalDragImageData
import swiftPMImport.co.typie.compose.NativeOptionalPageRenderInfo
import swiftPMImport.co.typie.compose.NativeOptionalString
import swiftPMImport.co.typie.compose.NativePageRenderInfo
import swiftPMImport.co.typie.compose.NativePageTexture

private inline fun <T> throwingCall(block: (errorPtr: kotlinx.cinterop.CPointer<ObjCObjectVar<NSError?>>) -> T): T =
  memScoped {
    val error = alloc<ObjCObjectVar<NSError?>>()
    val result = block(error.ptr)
    error.value?.let { throw EditorException(it.localizedDescription) }
    return result
  }

@Module
actual class EditorModule {
  @Single
  actual fun editorEngine(ctx: PlatformContext): EditorEngine {
    val path = NSBundle.mainBundle.pathForResource("icu", "zst")!!
    val icu = NSData.create(contentsOfFile = path)!!
    return IosEditorEngine(NativeEditorEngine().also {
      throwingCall { err -> it.loadIcuData(icu, err) }
    })
  }
}

private class IosEditorEngine(private val native: NativeEditorEngine) : EditorEngine {
  override suspend fun initGpu(): Boolean {
    // TODO: async Swift bridge — 현재는 CPU fallback
    return false
  }

  override fun validateRegex(pattern: String): Boolean {
    return throwingCall { err -> native.validateRegexWithPattern(pattern, err) }!!.boolValue
  }

  override fun createEditor(scaleFactor: Double, snapshot: ByteArray?): Editor {
    val editor = throwingCall { err ->
      native.createEditorWithScaleFactor(scaleFactor, snapshot = snapshot?.toNSData(), error = err)
    }!!
    return IosEditor(editor)
  }

  override fun addFontBase(family: String, weight: Int, data: ByteArray) {
    throwingCall { err -> native.addFontBaseWithFamily(family, weight = weight, data = data.toNSData(), error = err) }
  }

  override fun addFontChunk(family: String, weight: Int, data: ByteArray) {
    throwingCall { err -> native.addFontChunkWithFamily(family, weight = weight, data = data.toNSData(), error = err) }
  }

  override fun setAvailableFonts(fontsJson: String) {
    throwingCall { err -> native.setAvailableFontsWithFontsJson(fontsJson, err) }
  }

  override fun setTextReplacementRules(rulesJson: String) {
    throwingCall { err -> native.setTextReplacementRulesWithRulesJson(rulesJson, err) }
  }

  override fun getSlateOffsets(): String {
    return throwingCall { err -> native.getSlateOffsetsAndReturnError(err) }!!
  }

  override fun close() {}
}

private class IosEditor(private val native: NativeEditor) : Editor {
  override fun attachSurface(pageIndex: Int): PageTexture {
    val texture = throwingCall { err -> native.attachSurfaceWithPageIndex(pageIndex, error = err) }!!
    return IosPageTexture(texture)
  }

  override fun detachSurface(pageIndex: Int) {
    throwingCall { err -> native.detachSurfaceWithPageIndex(pageIndex, error = err) }
  }

  override fun dispatch(messageJson: String) {
    throwingCall { err -> native.dispatchWithMessageJson(messageJson, err) }
  }

  override fun tick() {
    throwingCall { err -> native.tickAndReturnError(err) }
  }

  override fun flush() {
    throwingCall { err -> native.flushAndReturnError(err) }
  }

  override fun getPageCount(): Int {
    return throwingCall { err -> native.getPageCountAndReturnError(err) }!!.intValue
  }

  override fun getRenderInfo(pageIndex: Int): PageRenderInfo? {
    return throwingCall { err -> native.getRenderInfoWithPageIndex(pageIndex, error = err) }!!.value?.toKotlin()
  }

  override fun renderDragImage(visiblePages: List<Int>, pageIdx: Int): DragImageData? {
    val pages = visiblePages.map { NSNumber(it) }
    return throwingCall { err -> native.renderDragImageWithVisiblePages(pages, pageIdx = pageIdx, error = err) }!!.value?.toKotlin()
  }

  override fun isSelectionHit(pageIdx: Int, x: Float, y: Float): Boolean {
    return throwingCall { err -> native.isSelectionHitWithPageIdx(pageIdx, x = x, y = y, error = err) }!!.boolValue
  }

  override fun isCursorHit(pageIdx: Int, x: Float, y: Float): Boolean {
    return throwingCall { err -> native.isCursorHitWithPageIdx(pageIdx, x = x, y = y, error = err) }!!.boolValue
  }

  override fun export(mode: Int, version: ByteArray?): ByteArray {
    return throwingCall { err -> native.exportWithMode(mode, version = version?.toNSData(), error = err) }!!.toByteArray()
  }

  override fun importUpdates(data: ByteArray) {
    throwingCall { err -> native.importUpdates(data.toNSData(), err) }
  }

  override fun importUpdatesBatch(updates: List<ByteArray>) {
    throwingCall { err -> native.importUpdatesBatch(updates.map { it.toNSData() }, err) }
  }

  override fun getCharacterCounts(): CharacterCounts {
    return throwingCall { err -> native.getCharacterCountsAndReturnError(err) }!!.toKotlin()
  }

  override fun getClipboardData(): String? {
    return throwingCall { err -> native.getClipboardDataAndReturnError(err) }!!.value
  }

  override fun getTextWithMappings(): String {
    return throwingCall { err -> native.getTextWithMappingsAndReturnError(err) }!!
  }

  override fun performSearch(query: String, matchWholeWord: Boolean): String {
    return throwingCall { err -> native.performSearchWithQuery(query, matchWholeWord = matchWholeWord, error = err) }!!
  }

  override fun setTrackedItems(group: Int, itemsJson: String) {
    throwingCall { err -> native.setTrackedItemsWithGroup(group, itemsJson = itemsJson, error = err) }
  }

  override fun removeTrackedItems(group: Int, idsJson: String) {
    throwingCall { err -> native.removeTrackedItemsWithGroup(group, idsJson = idsJson, error = err) }
  }

  override fun revealTrackedItem(group: Int, id: String): Boolean {
    return throwingCall { err -> native.revealTrackedItemWithGroup(group, id = id, error = err) }!!.boolValue
  }

  override fun replaceTextInBlock(blockId: String, startOffset: Int, endOffset: Int, replacement: String): Boolean {
    return throwingCall { err ->
      native.replaceTextInBlockWithBlockId(blockId, startOffset = startOffset, endOffset = endOffset, replacement = replacement, error = err)
    }!!.boolValue
  }

  override fun replaceTextInBlocks(itemsJson: String) {
    throwingCall { err -> native.replaceTextInBlocksWithItemsJson(itemsJson, err) }
  }

  override fun insertTemplateFragment(snapshot: ByteArray) {
    throwingCall { err -> native.insertTemplateFragment(snapshot.toNSData(), err) }
  }

  override fun setTracing(traceId: String, parentSpanId: String) {
    throwingCall { err -> native.setTracingWithTraceId(traceId, parentSpanId = parentSpanId, error = err) }
  }

  override fun clearTracing() {
    throwingCall { err -> native.clearTracingAndReturnError(err) }
  }

  override fun drainTraces(): String {
    return throwingCall { err -> native.drainTracesAndReturnError(err) }!!
  }

  override fun close() {}
}

private class IosPageTexture(private val native: NativePageTexture) : PageTexture {
  override val nativeHandle: Long = native.nativeHandle
  override val width: Int = native.width
  override val height: Int = native.height
  override fun pixelData(): ByteArray? = native.pixelData()?.toByteArray()
  override fun close() {} // iOS: ARC deinit으로 해제
}

private fun ByteArray.toNSData(): NSData = memScoped {
  NSData.create(bytes = allocArrayOf(this@toNSData), length = size.toULong())
}

private fun NSData.toByteArray(): ByteArray {
  val byteArray = ByteArray(length.toInt())
  byteArray.usePinned { pinned ->
    memcpy(pinned.addressOf(0), bytes, length)
  }
  return byteArray
}

private fun NativePageRenderInfo.toKotlin() = PageRenderInfo(
  width = width,
  height = height,
  bufferSize = bufferSize,
)

private fun NativeCharacterCounts.toKotlin() = CharacterCounts(
  docWithWhitespace = docWithWhitespace,
  docWithoutWhitespace = docWithoutWhitespace,
  docWithoutWhitespaceAndPunctuation = docWithoutWhitespaceAndPunctuation,
  selectionWithWhitespace = selectionWithWhitespace,
  selectionWithoutWhitespace = selectionWithoutWhitespace,
  selectionWithoutWhitespaceAndPunctuation = selectionWithoutWhitespaceAndPunctuation,
)

private fun NativeDragImageData.toKotlin() = DragImageData(
  width = width,
  height = height,
  offsetX = offsetX,
  offsetY = offsetY,
  scaleFactor = scaleFactor,
  pixels = pixels.toByteArray(),
)
