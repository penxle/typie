package co.typie.screen.editor.editor

import co.typie.editor.external.EditorEmbedAsset
import co.typie.editor.external.EditorExternalAsset
import co.typie.editor.external.EditorFileAsset
import co.typie.editor.external.EditorImageAsset
import co.typie.graphql.EditorScreen_PersistBlobAsFile_Mutation.PersistBlobAsFile
import co.typie.graphql.EditorScreen_PersistBlobAsImage_Mutation.PersistBlobAsImage
import co.typie.graphql.EditorScreen_UnfurlEmbed_Mutation.UnfurlEmbed
import co.typie.graphql.fragment.EditorExternalAsset_asset

internal fun EditorExternalAsset_asset.toEditorExternalAsset(): EditorExternalAsset? =
  when (__typename) {
    "Image" ->
      onImage?.let { image ->
        EditorImageAsset(
          id = image.id,
          url = image.url,
          width = image.width,
          height = image.height,
          ratio = image.ratio,
          placeholder = image.placeholder,
        )
      }
    "File" ->
      onFile?.let { file ->
        EditorFileAsset(id = file.id, name = file.name, url = file.url, size = file.size)
      }
    "Embed" ->
      onEmbed?.let { embed ->
        EditorEmbedAsset(
          id = embed.id,
          url = embed.url,
          title = embed.title,
          description = embed.description,
          thumbnailUrl = embed.thumbnailUrl,
          html = embed.html,
        )
      }
    else -> null
  }

internal fun PersistBlobAsImage.toEditorImageAsset(): EditorImageAsset =
  EditorImageAsset(
    id = id,
    url = url,
    width = width,
    height = height,
    ratio = ratio,
    placeholder = placeholder,
  )

internal fun PersistBlobAsFile.toEditorFileAsset(): EditorFileAsset =
  EditorFileAsset(id = id, name = name, url = url, size = size)

internal fun UnfurlEmbed.toEditorEmbedAsset(): EditorEmbedAsset =
  EditorEmbedAsset(
    id = id,
    url = url,
    title = title,
    description = description,
    thumbnailUrl = thumbnailUrl,
    html = html,
  )
