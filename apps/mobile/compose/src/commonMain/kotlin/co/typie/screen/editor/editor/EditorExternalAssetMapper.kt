package co.typie.screen.editor.editor

import co.typie.editor.external.EditorEmbedAsset
import co.typie.graphql.EditorScreen_UnfurlEmbed_Mutation.UnfurlEmbed

internal fun UnfurlEmbed.toEditorEmbedAsset(): EditorEmbedAsset =
  EditorEmbedAsset(
    id = id,
    url = url,
    title = title,
    description = description,
    thumbnailUrl = thumbnailUrl,
    html = html,
  )
