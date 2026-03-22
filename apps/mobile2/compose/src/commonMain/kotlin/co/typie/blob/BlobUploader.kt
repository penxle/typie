package co.typie.blob

import co.typie.graphql.Blob_IssueBlobUploadUrl_Mutation
import com.apollographql.apollo.ApolloClient
import io.ktor.client.HttpClient
import io.ktor.client.request.forms.formData
import io.ktor.client.request.forms.submitFormWithBinaryData
import io.ktor.http.Headers
import io.ktor.http.HttpHeaders
import org.koin.core.annotation.Single

@Single
class BlobUploader(
  private val apolloClient: ApolloClient,
  private val httpClient: HttpClient,
) {
  suspend fun uploadBytes(
    bytes: ByteArray,
    filename: String,
    mimeType: String?,
  ): String {
    val resolvedMimeType = mimeType ?: "application/octet-stream"
    val result = apolloClient.mutation(
      Blob_IssueBlobUploadUrl_Mutation(
        input = co.typie.graphql.type.IssueBlobUploadUrlInput(filename = filename),
      ),
    ).execute().dataOrThrow().issueBlobUploadUrl

    httpClient.submitFormWithBinaryData(
      url = result.url,
      formData = formData {
        result.fields.asStringMap().forEach { (key, value) ->
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

    return result.path
  }
}

private fun Any?.asStringMap(): Map<String, String> {
  val map = this as? Map<*, *> ?: error("Expected upload fields to be a JSON object.")
  return map.entries.associate { (key, value) ->
    key.toString() to value.toString()
  }
}
