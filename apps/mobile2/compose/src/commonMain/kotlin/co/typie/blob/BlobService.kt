package co.typie.blob

import co.typie.graphql.BlobService_IssueBlobUploadUrl_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.IssueBlobUploadUrlInput
import com.apollographql.apollo.ApolloClient
import io.ktor.client.HttpClient
import io.ktor.client.request.forms.formData
import io.ktor.client.request.forms.submitFormWithBinaryData
import io.ktor.http.Headers
import io.ktor.http.HttpHeaders
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.koin.core.annotation.Single

@Single
class BlobService(
  private val apolloClient: ApolloClient,
  private val httpClient: HttpClient,
) {
  suspend fun uploadBytes(
    bytes: ByteArray,
    filename: String,
    mimeType: String?,
  ): String {
    val resolvedMimeType = mimeType ?: "application/octet-stream"
    val result = apolloClient.executeMutation(
      BlobService_IssueBlobUploadUrl_Mutation(
        input = IssueBlobUploadUrlInput(filename = filename),
      ),
    )

    httpClient.submitFormWithBinaryData(
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
