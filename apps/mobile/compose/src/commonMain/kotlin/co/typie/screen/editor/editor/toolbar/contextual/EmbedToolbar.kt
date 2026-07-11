package co.typie.screen.editor.editor.toolbar.contextual

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import co.typie.editor.external.EditorEmbedAsset
import co.typie.editor.external.EditorEmbedUnfurl
import co.typie.editor.external.LocalEditorExternalElementState
import co.typie.editor.ffi.FlatImeOp
import co.typie.editor.ffi.Message
import co.typie.editor.ffi.NodeOp
import co.typie.editor.ffi.PlainNode
import co.typie.editor.runtime.LocalEditorRuntime
import co.typie.editor.scroll.EditorBringIntoViewTarget
import co.typie.editor.scroll.LocalEditorBringIntoViewRequests
import co.typie.editor.scroll.syncWithBringIntoView
import co.typie.graphql.Apollo
import co.typie.graphql.EditorScreen_UnfurlEmbed_Mutation
import co.typie.graphql.executeMutation
import co.typie.graphql.type.UnfurlEmbedInput
import co.typie.icons.Lucide
import co.typie.screen.editor.editor.toEditorEmbedAsset
import co.typie.screen.editor.editor.toolbar.EditorToolbarButton
import co.typie.screen.editor.editor.toolbar.EditorToolbarPage
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageKey
import co.typie.screen.editor.editor.toolbar.EditorToolbarPageScope
import co.typie.screen.editor.editor.toolbar.EditorToolbarRow
import co.typie.ui.component.LabelPosition
import co.typie.ui.component.Text
import co.typie.ui.component.TextField
import co.typie.ui.component.dialog.Dialog
import co.typie.ui.component.dialog.DialogActionButton
import co.typie.ui.component.dialog.DialogActionDivider
import co.typie.ui.component.dialog.DialogResult
import co.typie.ui.component.dialog.DialogScope
import co.typie.ui.component.dialog.LocalDialog
import co.typie.ui.component.dialog.dismiss
import co.typie.ui.component.dialog.resolve
import co.typie.ui.component.toast.LocalToast
import co.typie.ui.theme.AppShapes
import co.typie.ui.theme.AppTheme
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

internal fun editorEmbedToolbarPage(embed: PlainNode.Embed?, nodeId: String?): EditorToolbarPage =
  EditorToolbarPage(
    key = EditorToolbarPageKey.Embed,
    icon = Lucide.FileUp,
    contentDescription = "임베드 툴바",
    ownerNodeId = nodeId,
    content = { scope -> EditorEmbedToolbar(scope = scope, embed = embed, nodeId = nodeId) },
  )

@Composable
private fun EditorEmbedToolbar(
  scope: EditorToolbarPageScope,
  embed: PlainNode.Embed?,
  nodeId: String?,
  modifier: Modifier = Modifier,
) {
  val dialog = LocalDialog.current
  val toast = LocalToast.current
  val uriHandler = LocalUriHandler.current
  val runtime = LocalEditorRuntime.current
  val bringIntoViewRequests = LocalEditorBringIntoViewRequests.current
  val externalElementState = LocalEditorExternalElementState.current
  val embedState = externalElementState.embeds
  val embedId = embed?.id
  val asset = embedId?.let(embedState.assets::get)
  val unfurling = nodeId?.let { embedState.unfurls.containsKey(it) } == true
  val hasEmbed = embedId != null || unfurling

  EditorToolbarRow(scope = scope, modifier = modifier) {
    if (!hasEmbed) {
      EditorToolbarButton(
        icon = Lucide.FileUp,
        contentDescription = "임베드 삽입",
        onClick = {
          val selectedNodeId = nodeId ?: return@EditorToolbarButton
          scope.commandScope.launch {
            val input = dialog.promptEmbedUrl() ?: return@launch
            val url = normalizedEmbedUrl(input)
            val unfurl = EditorEmbedUnfurl()
            embedState.unfurls[selectedNodeId] = unfurl

            try {
              completeAttachmentOperation(
                persist = { unfurlEmbedAsset(url) },
                isCurrent = { embedState.unfurls[selectedNodeId] === unfurl },
                cache = externalElementState::put,
                commit = { embedded ->
                  val editor = checkNotNull(runtime.editor) { "No active editor is available" }
                  val committedState =
                    editor.syncWithBringIntoView(bringIntoViewRequests) {
                      if (editor.ime?.composing != null) {
                        enqueue(Message.TextInput(listOf(FlatImeOp.CommitAsIs)))
                      }
                      enqueue(
                        Message.Node(
                          NodeOp.SetAttrs(
                            id = selectedNodeId,
                            attrs = PlainNode.Embed(id = embedded.id),
                          )
                        )
                      )
                      beforeCommit { bringIntoView(EditorBringIntoViewTarget.CurrentSelectionHead) }
                    }
                  checkNotNull(committedState) { "Editor embed attrs did not commit" }
                },
                clearPending = {
                  if (embedState.unfurls[selectedNodeId] === unfurl) {
                    embedState.unfurls.remove(selectedNodeId)
                  }
                },
              )
            } catch (error: CancellationException) {
              throw error
            } catch (error: Throwable) {
              reportAttachmentFailure(AttachmentKind.Embed, error)
              toast.error("링크를 임베드할 수 없어요.")
            }
          }
        },
      )
    }
    if (asset != null) {
      EditorToolbarButton(
        icon = Lucide.ExternalLink,
        contentDescription = "외부 링크 열기",
        onClick = { uriHandler.openUri(asset.url) },
      )
    }
    EditorToolbarButton(
      icon = Lucide.Trash2,
      contentDescription = "임베드 삭제",
      onClick = {
        val selectedNodeId = nodeId
        if (selectedNodeId != null) {
          embedState.unfurls.remove(selectedNodeId)
          scope.sendMessage(Message.Node(NodeOp.Delete(id = selectedNodeId)))
        }
      },
    )
  }
}

private suspend fun Dialog.promptEmbedUrl(): String? {
  return when (val result = present<String>(dismissible = true) { EmbedUrlDialog() }) {
    is DialogResult.Resolved -> result.value
    DialogResult.Dismissed -> null
  }
}

@Composable
context(scope: DialogScope<String>)
private fun EmbedUrlDialog() {
  var url by remember { mutableStateOf("") }

  fun submit() {
    val trimmed = url.trim()
    if (trimmed.isNotEmpty()) {
      resolve(trimmed)
    }
  }

  Column(
    modifier =
      Modifier.widthIn(max = 340.dp)
        .clip(AppShapes.rounded(AppShapes.lg))
        .background(AppTheme.colors.surfaceDefault)
  ) {
    Column(Modifier.padding(start = 20.dp, end = 20.dp, top = 24.dp, bottom = 20.dp)) {
      Text("임베드 삽입", style = AppTheme.typography.title)
      Spacer(Modifier.height(16.dp))
      TextField(
        value = url,
        onValueChange = { url = it },
        label = "URL",
        placeholder = "https://...",
        labelPosition = LabelPosition.None,
        autoFocus = true,
        keyboardType = KeyboardType.Uri,
        imeAction = ImeAction.Done,
        onImeAction = ::submit,
        modifier = Modifier.fillMaxWidth(),
      )
    }

    Box(Modifier.fillMaxWidth().height(1.dp).background(AppTheme.colors.borderHairline))

    Row(Modifier.fillMaxWidth()) {
      DialogActionButton(text = "취소") { dismiss() }
      DialogActionDivider()
      DialogActionButton(text = "삽입") { submit() }
    }
  }
}

private fun normalizedEmbedUrl(input: String): String {
  val trimmed = input.trim()
  return if (Regex("^[^:]+://").containsMatchIn(trimmed)) trimmed else "https://$trimmed"
}

private suspend fun unfurlEmbedAsset(url: String): EditorEmbedAsset {
  val embed =
    Apollo.executeMutation(EditorScreen_UnfurlEmbed_Mutation(input = UnfurlEmbedInput(url = url)))
      .unfurlEmbed
  return embed.toEditorEmbedAsset()
}
