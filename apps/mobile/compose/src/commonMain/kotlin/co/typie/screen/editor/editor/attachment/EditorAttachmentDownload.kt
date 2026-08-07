package co.typie.screen.editor.editor.attachment

import io.ktor.client.HttpClient
import io.ktor.client.request.get
import io.ktor.client.statement.bodyAsBytes
import io.ktor.http.ContentDisposition
import io.ktor.http.ContentType
import io.ktor.http.HttpHeaders
import io.ktor.http.decodeURLPart
import io.ktor.http.fileExtensions

internal data class DownloadedEditorAttachment(
  val bytes: ByteArray,
  val filename: String,
  val mimeType: String,
)

internal suspend fun HttpClient.downloadEditorAttachment(
  url: String,
  fallbackFilename: String? = null,
  defaultFilenameStem: String = "file",
): DownloadedEditorAttachment {
  val response = get(url)
  val contentType =
    response.headers[HttpHeaders.ContentType]?.let {
      runCatching { ContentType.parse(it).withoutParameters() }.getOrNull()
    } ?: ContentType.Application.OctetStream
  val filename =
    parseAttachmentFilename(response.headers[HttpHeaders.ContentDisposition])
      ?: fallbackFilename?.trim()?.takeIf(String::isNotEmpty)
      ?: defaultAttachmentFilename(defaultFilenameStem, contentType)

  return DownloadedEditorAttachment(
    bytes = response.bodyAsBytes(),
    filename = filename,
    mimeType = contentType.toString(),
  )
}

internal fun parseAttachmentFilename(contentDisposition: String?): String? {
  val parsed =
    contentDisposition?.let { runCatching { ContentDisposition.parse(it) }.getOrNull() }
      ?: return null

  return parsed
    .parameter(ContentDisposition.Parameters.FileNameAsterisk)
    ?.decodeExtendedFilename()
    ?.trim()
    ?.takeIf(String::isNotEmpty)
    ?: parsed.parameter(ContentDisposition.Parameters.FileName)?.trim()?.takeIf(String::isNotEmpty)
}

private fun String.decodeExtendedFilename(): String? {
  val charsetEnd = indexOf('\'')
  if (charsetEnd <= 0 || !substring(0, charsetEnd).equals("UTF-8", ignoreCase = true)) {
    return null
  }
  val languageEnd = indexOf('\'', startIndex = charsetEnd + 1)
  if (languageEnd < 0) return null

  return runCatching { substring(languageEnd + 1).decodeURLPart() }.getOrNull()
}

private fun defaultAttachmentFilename(stem: String, contentType: ContentType): String {
  val normalizedStem = stem.trim().ifEmpty { "file" }
  val extension = contentType.fileExtensions().firstOrNull() ?: return normalizedStem
  return "$normalizedStem.$extension"
}
