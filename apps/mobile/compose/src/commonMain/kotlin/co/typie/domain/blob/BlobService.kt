package co.typie.domain.blob

import co.typie.graphql.Apollo
import co.typie.graphql.BlobService_IssueBlobUploadUrl_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.IssueBlobUploadUrlInput
import co.typie.network.Http
import io.ktor.client.request.forms.formData
import io.ktor.client.request.forms.submitFormWithBinaryData
import io.ktor.http.HttpHeaders
import io.ktor.http.headers
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

internal enum class BlobUploadStage {
  IssueUploadUrl,
  TransferBytes,
}

internal class BlobUploadException(val stage: BlobUploadStage, cause: Throwable) :
  RuntimeException("Blob upload failed at $stage", cause)

object BlobService {
  suspend fun uploadBytes(bytes: ByteArray, filename: String, mimeType: String?): String {
    val resolvedMimeType = mimeType ?: "application/octet-stream"
    val result =
      withBlobUploadStage(BlobUploadStage.IssueUploadUrl) {
        Apollo.executeMutation(
          BlobService_IssueBlobUploadUrl_Mutation(
            input = IssueBlobUploadUrlInput(filename = filename)
          )
        )
      }

    withBlobUploadStage(BlobUploadStage.TransferBytes) {
      Http.submitFormWithBinaryData(
        url = result.issueBlobUploadUrl.url,
        formData =
          formData {
            result.issueBlobUploadUrl.fields.jsonObject.asStringMap().forEach {
              append(it.key, it.value)
            }

            append("Content-Type", resolvedMimeType)

            append(
              key = "file",
              value = bytes,
              headers =
                headers {
                  append(HttpHeaders.ContentType, resolvedMimeType)
                  append(HttpHeaders.ContentDisposition, """filename="$filename"""")
                },
            )
          },
      )
    }

    return result.issueBlobUploadUrl.path
  }
}

private suspend inline fun <T> withBlobUploadStage(stage: BlobUploadStage, block: () -> T): T =
  try {
    block()
  } catch (error: CancellationException) {
    throw error
  } catch (error: Throwable) {
    throw BlobUploadException(stage = stage, cause = error)
  }

private fun JsonObject.asStringMap(): Map<String, String> {
  return mapValues { it.value.jsonPrimitive.content }
}
