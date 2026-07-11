package co.typie.domain.blob

import co.typie.graphql.Apollo
import co.typie.graphql.BlobService_IssueBlobUploadUrl_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.IssueBlobUploadUrlInput
import co.typie.network.Http
import co.typie.platform.PickedFile
import io.ktor.client.request.forms.formData
import io.ktor.client.request.forms.submitFormWithBinaryData
import io.ktor.http.HttpHeaders
import io.ktor.http.headers
import io.ktor.http.quote
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Semaphore
import kotlinx.coroutines.sync.withPermit
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
  private const val MaxConcurrentUploads = 5
  private val uploadSemaphore = Semaphore(MaxConcurrentUploads)

  suspend fun upload(file: PickedFile): String = uploadSemaphore.withPermit {
    val resolvedMimeType = file.mimeType ?: "application/octet-stream"
    val result =
      withBlobUploadStage(BlobUploadStage.IssueUploadUrl) {
        Apollo.executeMutation(
          BlobService_IssueBlobUploadUrl_Mutation(
            input = IssueBlobUploadUrlInput(filename = file.filename)
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

            appendInput(
              key = "file",
              headers =
                headers {
                  append(HttpHeaders.ContentType, resolvedMimeType)
                  append(HttpHeaders.ContentDisposition, "filename=${file.filename.quote()}")
                },
              size = file.size,
              block = file::openSource,
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
