package co.typie.platform

internal object NoopClipboard : Clipboard {
  override suspend fun copy(bytes: ByteArray, mimeType: String): Boolean = false

  override suspend fun copy(text: String, mimeType: String): Boolean = false

  override suspend fun copyRichText(html: String, text: String): Boolean = false

  override suspend fun paste(): ClipboardReadPayload? = null
}
