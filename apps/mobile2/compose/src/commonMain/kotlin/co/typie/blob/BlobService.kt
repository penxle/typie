package co.typie.blob

import co.typie.graphql.Apollo
import co.typie.graphql.BlobService_IssueBlobUploadUrl_Mutation
import co.typie.graphql.Http
import co.typie.graphql.executeMutation
import co.typie.graphql.type.IssueBlobUploadUrlInput
import io.ktor.client.request.forms.formData
import io.ktor.client.request.forms.submitFormWithBinaryData
import io.ktor.http.Headers
import io.ktor.http.HttpHeaders
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

object BlobService {
  suspend fun uploadBytes(
    bytes: ByteArray,
    filename: String,
    mimeType: String?,
  ): String {
    val resolvedMimeType = mimeType ?: "application/octet-stream"
    val result = Apollo.executeMutation(
      BlobService_IssueBlobUploadUrl_Mutation(
        input = IssueBlobUploadUrlInput(filename = filename),
      ),
    )

    Http.submitFormWithBinaryData(
      url = result.issueBlobUploadUrl.url,
      formData = formData {
        result.issueBlobUploadUrl.fields.jsonObject.asStringMap().forEach { (key, value) ->
          append(key, value)
        }
        append("Content-Type", resolvedMimeType)
        append(
          key = "file",
          value = bytes,
          headers = Headers.build {
            append(HttpHeaders.ContentType, resolvedMimeType)
            append(HttpHeaders.ContentDisposition, """filename="$filename"""")
          },
        )
      },
    )

    return result.issueBlobUploadUrl.path
  }
}

private fun JsonObject.asStringMap(): Map<String, String> {
  return mapValues { (_, value) -> value.jsonPrimitive.content }
}
