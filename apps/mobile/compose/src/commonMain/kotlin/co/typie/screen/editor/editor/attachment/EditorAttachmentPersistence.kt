package co.typie.screen.editor.editor.attachment

import co.typie.domain.blob.BlobService
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorImageAsset
import co.typie.graphql.Apollo
import co.typie.graphql.EditorScreen_PersistBlobAsFile_Mutation
import co.typie.graphql.EditorScreen_PersistBlobAsImage_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.PersistBlobAsFileInput
import co.typie.graphql.type.PersistBlobAsImageInput
import co.typie.platform.PickedFile
import co.typie.screen.editor.editor.toEditorFileAsset
import co.typie.screen.editor.editor.toEditorImageAsset

internal interface EditorAttachmentPersistence {
  suspend fun persistImage(file: PickedFile): EditorImageAsset

  suspend fun persistFile(file: PickedFile): EditorFileAsset
}

internal object GraphqlEditorAttachmentPersistence : EditorAttachmentPersistence {
  override suspend fun persistImage(file: PickedFile): EditorImageAsset {
    val path = BlobService.upload(file)
    return Apollo.executeMutation(
        EditorScreen_PersistBlobAsImage_Mutation(input = PersistBlobAsImageInput(path = path))
      )
      .persistBlobAsImage
      .toEditorImageAsset()
  }

  override suspend fun persistFile(file: PickedFile): EditorFileAsset {
    val path = BlobService.upload(file)
    return Apollo.executeMutation(
        EditorScreen_PersistBlobAsFile_Mutation(input = PersistBlobAsFileInput(path = path))
      )
      .persistBlobAsFile
      .toEditorFileAsset()
  }
}
