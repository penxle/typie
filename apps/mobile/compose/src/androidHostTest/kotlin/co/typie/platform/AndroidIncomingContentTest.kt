package co.typie.platform

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.test.runTest
import kotlinx.io.Buffer

class AndroidIncomingContentTest {
  private data class Item(
    val source: Source,
    val html: String? = null,
    val rawText: String? = null,
    val coercedText: String? = null,
    val attachment: IncomingContentItem? = null,
    val unreadable: Boolean = false,
  )

  private enum class Source {
    Uri,
    Intent,
    Other,
  }

  @Test
  fun uriAndIntentUseCoercedTextWhenRawTextIsAbsent() = runTest {
    for (source in listOf(Source.Uri, Source.Intent)) {
      val candidates =
        readAndroidIncomingContent(
          clipItems = listOf(Item(source = source, coercedText = "$source fallback")),
          html = Item::html,
          rawText = Item::rawText,
          coerceText = Item::coercedText,
          materialize = { item -> item.attachment },
        )

      assertEquals("$source fallback", candidates?.text)
      candidates?.close()
    }
  }

  @Test
  fun nonemptyHtmlSkipsAndroidAttachmentMaterialization() = runTest {
    var materializations = 0

    val candidates =
      readAndroidIncomingContent(
        clipItems = listOf(Item(source = Source.Other, html = "<p>rich</p>")),
        html = Item::html,
        rawText = Item::rawText,
        coerceText = Item::coercedText,
        materialize = {
          materializations += 1
          it.attachment
        },
      )

    assertEquals(0, materializations)
    assertEquals("<p>rich</p>", candidates?.html)
    candidates?.close()
  }

  @Test
  fun unreadableAttachmentDoesNotDiscardReadableSiblings() = runTest {
    val readable = imageItem {}
    val candidates =
      readAndroidIncomingContent(
        clipItems =
          listOf(
            Item(source = Source.Uri, attachment = readable),
            Item(source = Source.Uri, unreadable = true),
          ),
        html = Item::html,
        rawText = Item::rawText,
        coerceText = Item::coercedText,
        materialize = { item ->
          if (item.unreadable) error("unreadable")
          item.attachment
        },
      )

    assertEquals(listOf(readable), candidates?.items)
    assertEquals(1, candidates?.unreadableItemCount)
    candidates?.close()
  }

  @Test
  fun cancellationClosesPartiallyLoadedFilesExactlyOnce() = runTest {
    var releases = 0
    val firstLoaded = CompletableDeferred<Unit>()
    val never = CompletableDeferred<Unit>()
    val loading = async {
      loadOwnedIncomingContentItems(
        listOf(
          { imageItem { releases += 1 }.also { firstLoaded.complete(Unit) } },
          {
            never.await()
            null
          },
        )
      )
    }
    firstLoaded.await()

    loading.cancelAndJoin()

    assertEquals(1, releases)
  }

  @Test
  fun siblingFailurePreservesLoadedFilesAndCountsUnreadable() = runTest {
    var releases = 0
    val firstLoaded = CompletableDeferred<Unit>()

    val loaded =
      loadOwnedIncomingContentItems(
        listOf(
          { imageItem { releases += 1 }.also { firstLoaded.complete(Unit) } },
          {
            firstLoaded.await()
            error("provider failed")
          },
        )
      )

    assertEquals(1, loaded.unreadableItemCount)
    loaded.items.forEach { it.file.close() }
    assertEquals(1, releases)
  }

  private fun imageItem(onRelease: () -> Unit): IncomingContentItem =
    IncomingContentItem(
      kind = IncomingContentItem.Kind.Image,
      file =
        PickedFile(
          filename = "image.png",
          mimeType = "image/png",
          size = 1,
          previewModel = Unit,
          imageWidth = 1,
          imageHeight = 1,
          openSource = { Buffer() },
          release = onRelease,
        ),
    )
}
