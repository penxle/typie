package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import kotlinx.io.Buffer

class IncomingContentTest {
  @Test
  fun nonemptyHtmlSkipsAttachmentMaterialization() = runTest {
    var materializations = 0

    val candidates =
      materializeIncomingContentCandidates(html = "<p>rich</p>", text = "rich") {
        materializations += 1
        LoadedIncomingContentItems(listOf(trackedItem(IncomingContentItem.Kind.Image).item))
      }

    assertEquals(0, materializations)
    assertEquals("<p>rich</p>", candidates?.html)
    candidates?.close()
  }

  @Test
  fun cancellationClosesPartiallyLoadedItemsExactlyOnce() = runTest {
    val item = trackedItem(IncomingContentItem.Kind.Image)
    val firstLoaded = CompletableDeferred<Unit>()
    val never = CompletableDeferred<Unit>()
    val loading = async {
      loadOwnedIncomingContentItems(
        listOf(
          { item.item.also { firstLoaded.complete(Unit) } },
          {
            never.await()
            null
          },
        )
      )
    }
    firstLoaded.await()

    loading.cancelAndJoin()

    assertEquals(1, item.releaseCount)
  }

  @Test
  fun cancellationBeforeCrossDispatcherDeliveryClosesLoadedCandidatesExactlyOnce() = runTest {
    val item = trackedItem(IncomingContentItem.Kind.Image)
    val loaded = CompletableDeferred<Unit>()
    val loading = async {
      loadOwnedIncomingContentCandidates(StandardTestDispatcher(testScheduler)) {
        IncomingContentCandidates(items = listOf(item.item)).also { loaded.complete(Unit) }
      }
    }
    loaded.await()

    loading.cancelAndJoin()

    assertEquals(1, item.releaseCount)
  }

  @Test
  fun siblingFailurePreservesLoadedItemsAndCountsUnreadable() = runTest {
    val item = trackedItem(IncomingContentItem.Kind.Image)
    val firstLoaded = CompletableDeferred<Unit>()

    val loaded =
      loadOwnedIncomingContentItems(
        listOf(
          { item.item.also { firstLoaded.complete(Unit) } },
          {
            firstLoaded.await()
            error("provider failed")
          },
        )
      )

    assertEquals(1, loaded.unreadableItemCount)
    assertEquals(listOf(item.item), loaded.items)
    loaded.items.forEach { it.file.close() }
    assertEquals(1, item.releaseCount)
  }

  @Test
  fun anyNonemptyHtmlWinsOverNativeAttachments() {
    val image = trackedItem(IncomingContentItem.Kind.Image)
    val file = trackedItem(IncomingContentItem.Kind.File)
    val candidates =
      IncomingContentCandidates(
        html = "<meta data-slice-v2=\"valid\"><p>mixed</p>",
        text = "mixed",
        items = listOf(image.item, file.item),
      )

    val selected = candidates.select(IncomingContentMode.Rich)

    assertEquals(SelectedIncomingContent.RichText(candidates.html, "mixed"), selected)
    assertEquals(1, image.releaseCount)
    assertEquals(1, file.releaseCount)
  }

  @Test
  fun closingUnselectedCandidatesReleasesItemsExactlyOnce() {
    val image = trackedItem(IncomingContentItem.Kind.Image)
    val candidates = IncomingContentCandidates(items = listOf(image.item))

    candidates.close()
    candidates.close()

    assertEquals(1, image.releaseCount)
    assertFailsWith<IllegalStateException> { candidates.select(IncomingContentMode.Rich) }
  }

  @Test
  fun externalHtmlWinsOverNativeAttachmentsWithoutLosingItsSupportedContent() {
    val file = trackedItem(IncomingContentItem.Kind.File, "first.pdf")
    val image = trackedItem(IncomingContentItem.Kind.Image, "second.png")
    val candidates =
      IncomingContentCandidates(
        html = "<p>caption<img src=\"cid:image\"></p>",
        text = "caption",
        items = listOf(file.item, image.item),
      )

    val selected = candidates.select(IncomingContentMode.Rich)

    assertEquals(SelectedIncomingContent.RichText(candidates.html, "caption"), selected)
    assertEquals(1, file.releaseCount)
    assertEquals(1, image.releaseCount)
  }

  @Test
  fun ordinaryHtmlWithoutAttachmentsRemainsRichText() {
    val candidates = IncomingContentCandidates(html = "<p>linked file</p>", text = "linked file")

    val selected = candidates.select(IncomingContentMode.Rich)

    assertEquals(SelectedIncomingContent.RichText("<p>linked file</p>", "linked file"), selected)
  }

  @Test
  fun externalHtmlWithEmbeddedImageKeepsTheExistingRichTextPastePath() {
    val candidates =
      IncomingContentCandidates(
        html = "<p>before<img src=\"https://example.com/image.png\">after</p>",
        text = "before after",
      )

    val selected = candidates.select(IncomingContentMode.Rich)

    assertEquals(SelectedIncomingContent.RichText(candidates.html, "before after"), selected)
  }

  @Test
  fun attachmentOnlyPayloadTransfersItemsInProviderOrder() {
    val file = trackedItem(IncomingContentItem.Kind.File, "first.pdf")
    val image = trackedItem(IncomingContentItem.Kind.Image, "second.png")
    val candidates = IncomingContentCandidates(items = listOf(file.item, image.item))

    val selected = candidates.select(IncomingContentMode.Rich)

    val attachments = assertIs<SelectedIncomingContent.Attachments>(selected)
    assertEquals(listOf(file.item, image.item), attachments.items)
    assertEquals(0, file.releaseCount)
    assertEquals(0, image.releaseCount)

    attachments.close()
    attachments.close()
    assertEquals(1, file.releaseCount)
    assertEquals(1, image.releaseCount)
  }

  @Test
  fun attachmentSelectionPreservesUnreadableItemCount() {
    val image = trackedItem(IncomingContentItem.Kind.Image)
    val candidates = IncomingContentCandidates(items = listOf(image.item), unreadableItemCount = 1)

    val selected = candidates.select(IncomingContentMode.Rich)

    val attachments = assertIs<SelectedIncomingContent.Attachments>(selected)
    assertEquals(1, attachments.unreadableItemCount)
    attachments.close()
  }

  @Test
  fun unreadableOnlyPayloadStillSelectsAttachmentsForFailureReporting() {
    val candidates = IncomingContentCandidates(unreadableItemCount = 2)

    val selected = candidates.select(IncomingContentMode.Rich)

    val attachments = assertIs<SelectedIncomingContent.Attachments>(selected)
    assertEquals(emptyList(), attachments.items)
    assertEquals(2, attachments.unreadableItemCount)
  }

  @Test
  fun plainTextModeIgnoresHtmlAndReleasesAttachments() {
    val image = trackedItem(IncomingContentItem.Kind.Image)
    val candidates =
      IncomingContentCandidates(
        html = "<meta data-slice-v2=\"valid\"><p>rich</p>",
        text = "plain",
        items = listOf(image.item),
      )

    val selected = candidates.select(IncomingContentMode.PlainTextOnly)

    assertEquals(SelectedIncomingContent.RichText(html = null, text = "plain"), selected)
    assertEquals(1, image.releaseCount)
  }

  @Test
  fun emptyCandidatesSelectNoneAndSelectionIsSingleUse() {
    val candidates = IncomingContentCandidates(html = "", text = "")

    assertEquals(SelectedIncomingContent.None, candidates.select(IncomingContentMode.Rich))
    assertFailsWith<IllegalStateException> { candidates.select(IncomingContentMode.Rich) }
  }

  private fun trackedItem(
    kind: IncomingContentItem.Kind,
    filename: String = if (kind == IncomingContentItem.Kind.Image) "image.png" else "file.pdf",
  ): TrackedItem {
    var releaseCount = 0
    val pickedFile =
      PickedFile(
        filename = filename,
        mimeType = if (kind == IncomingContentItem.Kind.Image) "image/png" else "application/pdf",
        size = 1,
        previewModel = Unit,
        openSource = { Buffer() },
        release = { releaseCount += 1 },
      )
    return TrackedItem(
      item = IncomingContentItem(kind = kind, file = pickedFile),
      releaseCountProvider = { releaseCount },
    )
  }

  private class TrackedItem(
    val item: IncomingContentItem,
    private val releaseCountProvider: () -> Int,
  ) {
    val releaseCount: Int
      get() = releaseCountProvider()
  }
}
