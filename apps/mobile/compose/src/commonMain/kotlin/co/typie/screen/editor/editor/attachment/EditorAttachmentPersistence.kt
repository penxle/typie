package co.typie.screen.editor.editor.attachment

import co.typie.domain.blob.BlobService
import co.typie.domain.blob.asStringMap
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorImageAsset
import co.typie.graphql.Apollo
import co.typie.graphql.BlobService_AbandonBlobUpload_Mutation
import co.typie.graphql.BlobService_FinalizeBlobUploadAsFile_Mutation
import co.typie.graphql.BlobService_FinalizeBlobUploadAsImage_Mutation
import co.typie.graphql.BlobService_ReserveBlobUploads_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.AbandonBlobUploadInput
import co.typie.graphql.type.BlobUploadKind
import co.typie.graphql.type.FinalizeBlobUploadAsFileInput
import co.typie.graphql.type.FinalizeBlobUploadAsImageInput
import co.typie.graphql.type.ReserveBlobUploadItemInput
import co.typie.graphql.type.ReserveBlobUploadsInput
import co.typie.platform.IncomingContentItem
import co.typie.platform.PickedFile
import com.apollographql.apollo.api.Optional
import kotlinx.serialization.json.jsonObject

internal data class EditorAttachmentReserveItem(
  val kind: IncomingContentItem.Kind,
  val name: String,
  val format: String,
  val size: Long,
  val assetId: String? = null,
)

internal data class EditorAttachmentReservation(
  val assetId: String,
  val nonce: String,
  val path: String,
  val uploadUrl: String,
  val uploadFields: Map<String, String>,
)

internal interface EditorAttachmentPersistence {
  suspend fun reserve(
    documentId: String,
    items: List<EditorAttachmentReserveItem>,
  ): List<EditorAttachmentReservation>

  suspend fun upload(reservation: EditorAttachmentReservation, source: PickedFile)

  suspend fun finalizeImage(assetId: String, nonce: String): EditorImageAsset

  suspend fun finalizeFile(assetId: String, nonce: String): EditorFileAsset

  suspend fun abandon(assetId: String, nonce: String): Boolean
}

internal object GraphqlEditorAttachmentPersistence : EditorAttachmentPersistence {
  override suspend fun reserve(
    documentId: String,
    items: List<EditorAttachmentReserveItem>,
  ): List<EditorAttachmentReservation> =
    Apollo.executeMutation(
        BlobService_ReserveBlobUploads_Mutation(
          input =
            ReserveBlobUploadsInput(
              documentId = documentId,
              items =
                items.map { item ->
                  ReserveBlobUploadItemInput(
                    assetId = Optional.presentIfNotNull(item.assetId),
                    format = item.format,
                    kind =
                      when (item.kind) {
                        IncomingContentItem.Kind.Image -> BlobUploadKind.IMAGE
                        IncomingContentItem.Kind.File -> BlobUploadKind.FILE
                      },
                    name = item.name,
                    size = item.size,
                  )
                },
            )
        )
      )
      .reserveBlobUploads
      .map { reservation ->
        EditorAttachmentReservation(
          assetId = reservation.assetId,
          nonce = reservation.nonce,
          path = reservation.path,
          uploadUrl = reservation.uploadUrl,
          uploadFields = reservation.uploadFields.jsonObject.asStringMap(),
        )
      }

  override suspend fun upload(reservation: EditorAttachmentReservation, source: PickedFile) {
    BlobService.uploadToReservation(
      uploadUrl = reservation.uploadUrl,
      uploadFields = reservation.uploadFields,
      file = source,
    )
  }

  override suspend fun finalizeImage(assetId: String, nonce: String): EditorImageAsset =
    Apollo.executeMutation(
        BlobService_FinalizeBlobUploadAsImage_Mutation(
          input = FinalizeBlobUploadAsImageInput(assetId = assetId, nonce = nonce)
        )
      )
      .finalizeBlobUploadAsImage
      .let { image ->
        EditorImageAsset(
          id = image.id,
          url = image.url,
          width = image.width,
          height = image.height,
          ratio = image.ratio,
          placeholder = image.placeholder,
        )
      }

  override suspend fun finalizeFile(assetId: String, nonce: String): EditorFileAsset =
    Apollo.executeMutation(
        BlobService_FinalizeBlobUploadAsFile_Mutation(
          input = FinalizeBlobUploadAsFileInput(assetId = assetId, nonce = nonce)
        )
      )
      .finalizeBlobUploadAsFile
      .let { file ->
        EditorFileAsset(id = file.id, name = file.name, url = file.url, size = file.size)
      }

  override suspend fun abandon(assetId: String, nonce: String): Boolean =
    Apollo.executeMutation(
        BlobService_AbandonBlobUpload_Mutation(
          input = AbandonBlobUploadInput(assetId = assetId, nonce = nonce)
        )
      )
      .abandonBlobUpload
}
