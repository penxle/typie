package co.typie.platform

interface Share {
  suspend fun share(bytes: ByteArray, mimeType: String): Boolean
  suspend fun share(text: String): Boolean
}
