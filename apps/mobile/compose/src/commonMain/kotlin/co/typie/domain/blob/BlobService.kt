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

internal const val DEFAULT_BLOB_CONTENT_TYPE = "application/octet-stream"
internal const val BLOB_CONTENT_TYPE_FIELD = "Content-Type"

internal fun normalizeBlobContentType(mimeType: String?): String =
  mimeType?.trim()?.takeIf(String::isNotEmpty) ?: DEFAULT_BLOB_CONTENT_TYPE

internal data class BlobUploadForm(
  val fields: Map<String, String>,
  val filePartContentType: String,
  val filename: String,
)

internal fun blobUploadForm(uploadFields: Map<String, String>, file: PickedFile): BlobUploadForm {
  val contentType = normalizeBlobContentType(file.mimeType)
  return BlobUploadForm(
    fields =
      if (uploadFields.containsKey(BLOB_CONTENT_TYPE_FIELD)) {
        uploadFields
      } else {
        uploadFields + (BLOB_CONTENT_TYPE_FIELD to contentType)
      },
    filePartContentType = contentType,
    filename = file.filename,
  )
}

object BlobService {
  private const val MaxConcurrentUploads = 5
  private val uploadSemaphore = Semaphore(MaxConcurrentUploads)

  suspend fun upload(file: PickedFile): String = uploadSemaphore.withPermit {
    val result =
      withBlobUploadStage(BlobUploadStage.IssueUploadUrl) {
        Apollo.executeMutation(
          BlobService_IssueBlobUploadUrl_Mutation(
            input = IssueBlobUploadUrlInput(filename = file.filename)
          )
        )
      }

    transfer(
      uploadUrl = result.issueBlobUploadUrl.url,
      uploadFields = result.issueBlobUploadUrl.fields.jsonObject.asStringMap(),
      file = file,
    )

    return result.issueBlobUploadUrl.path
  }

  internal suspend fun uploadToReservation(
    uploadUrl: String,
    uploadFields: Map<String, String>,
    file: PickedFile,
  ) {
    transfer(uploadUrl = uploadUrl, uploadFields = uploadFields, file = file)
  }

  private suspend fun transfer(
    uploadUrl: String,
    uploadFields: Map<String, String>,
    file: PickedFile,
  ) {
    val form = blobUploadForm(uploadFields, file)
    withBlobUploadStage(BlobUploadStage.TransferBytes) {
      Http.submitFormWithBinaryData(
        url = uploadUrl,
        formData =
          formData {
            form.fields.forEach { append(it.key, it.value) }

            appendInput(
              key = "file",
              headers =
                headers {
                  append(HttpHeaders.ContentType, form.filePartContentType)
                  append(HttpHeaders.ContentDisposition, "filename=${form.filename.quote()}")
                },
              size = file.size,
              block = file::openSource,
            )
          },
      )
    }
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

internal fun JsonObject.asStringMap(): Map<String, String> {
  return mapValues { it.value.jsonPrimitive.content }
}
