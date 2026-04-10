package co.typie.platform

interface Clipboard {
  suspend fun copy(bytes: ByteArray, mimeType: String): Boolean

  suspend fun copy(text: String, mimeType: String): Boolean
}
