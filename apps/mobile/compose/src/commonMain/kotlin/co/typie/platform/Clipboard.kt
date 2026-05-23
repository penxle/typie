package co.typie.platform

interface Clipboard {
  suspend fun copy(bytes: ByteArray, mimeType: String): Boolean

  suspend fun copy(text: String, mimeType: String): Boolean

  suspend fun copyRichText(html: String, text: String): Boolean

  suspend fun paste(): ClipboardReadPayload?
}

data class ClipboardReadPayload(val html: String?, val text: String)
