package co.typie.entity_transfer

import co.typie.domain.entitytransfer.EntityClipboardMode
import co.typie.domain.entitytransfer.EntityClipboardService
import co.typie.domain.entitytransfer.EntityClipboardState
import co.typie.domain.entitytransfer.EntityPasteTarget
import co.typie.domain.entitytransfer.EntityTransferMaxDepth
import co.typie.domain.entitytransfer.EntityTransferSource
import co.typie.domain.entitytransfer.PasteError
import co.typie.result.Result
import kotlin.test.AfterTest
import kotlin.test.BeforeTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlinx.coroutines.runBlocking

class EntityClipboardServiceTest {
  @BeforeTest
  fun setUp() {
    EntityClipboardService.clear()
  }

  @Test
  fun `setCopy stores clipboard state`() {
    val item = EntityTransferSource.Document(id = "document-1", title = "문서", depth = 2)

    EntityClipboardService.setCopy(sourceSiteId = "site-a", items = listOf(item))

    assertEquals(
      EntityClipboardState(
        mode = EntityClipboardMode.Copy,
        sourceSiteId = "site-a",
        items = listOf(item),
      ),
      EntityClipboardService.currentState,
    )
  }

  @Test
  fun `blank source or empty items clears clipboard state`() {
    val item = EntityTransferSource.Document(id = "document-1", title = "문서", depth = 1)

    EntityClipboardService.setCopy(sourceSiteId = "site-a", items = listOf(item))
    EntityClipboardService.setCopy(sourceSiteId = "", items = listOf(item))
    assertNull(EntityClipboardService.currentState)

    EntityClipboardService.setCut(sourceSiteId = "site-a", items = listOf(item))
    EntityClipboardService.setCut(sourceSiteId = "site-a", items = emptyList())
    assertNull(EntityClipboardService.currentState)
  }

  @Test
  fun `cannot paste folder into itself descendants or too-deep destinations`() {
    val source =
      EntityTransferSource.Folder(
        id = "folder-a",
        title = "폴더",
        depth = 3,
        maxDescendantFoldersDepth = 4,
      )

    EntityClipboardService.setCut(sourceSiteId = "site-a", items = listOf(source))

    assertFalse(
      EntityClipboardService.canPaste(
        folderTarget(siteId = "site-a", destinationEntityId = "folder-a", destinationDepth = 3)
      )
    )
    assertFalse(
      EntityClipboardService.canPaste(
        folderTarget(
          siteId = "site-a",
          destinationEntityId = "folder-child",
          destinationDepth = 4,
          ancestorFolderIds = setOf("folder-a"),
        )
      )
    )
    assertFalse(
      EntityClipboardService.canPaste(
        folderTarget(
          siteId = "site-a",
          destinationEntityId = "folder-deep",
          destinationDepth = EntityTransferMaxDepth,
        )
      )
    )
  }

  @Test
  fun `pasteInto returns zero when clipboard is empty`() = runBlocking {
    var settled: Result<Int, PasteError>? = null

    EntityClipboardService.pasteInto(rootTarget(siteId = "site-a"))
      .collect(onPending = {}, onSettled = { settled = it })

    assertIs<Result.Ok<Int>>(settled)
    assertEquals(0, (settled as Result.Ok).value)
  }

  @Test
  fun `pasteInto returns zero without touching clipboard when target is invalid`() = runBlocking {
    val source =
      EntityTransferSource.Folder(
        id = "folder-a",
        title = "폴더",
        depth = 3,
        maxDescendantFoldersDepth = 4,
      )
    var settled: Result<Int, PasteError>? = null

    EntityClipboardService.setCut(sourceSiteId = "site-a", items = listOf(source))

    EntityClipboardService.pasteInto(
        folderTarget(siteId = "site-a", destinationEntityId = "folder-a", destinationDepth = 3)
      )
      .collect(onPending = {}, onSettled = { settled = it })

    assertIs<Result.Ok<Int>>(settled)
    assertEquals(0, (settled as Result.Ok).value)
    assertEquals(EntityClipboardMode.Cut, EntityClipboardService.currentState?.mode)
  }

  @AfterTest
  fun tearDown() {
    EntityClipboardService.clear()
  }
}

private fun rootTarget(siteId: String, lowerOrder: String? = null): EntityPasteTarget {
  return EntityPasteTarget(
    siteId = siteId,
    destinationEntityId = null,
    destinationDepth = -1,
    ancestorFolderIds = emptySet(),
    lowerOrder = lowerOrder,
    upperOrder = null,
  )
}

private fun folderTarget(
  siteId: String,
  destinationEntityId: String,
  destinationDepth: Int,
  ancestorFolderIds: Set<String> = emptySet(),
  lowerOrder: String? = null,
): EntityPasteTarget {
  return EntityPasteTarget(
    siteId = siteId,
    destinationEntityId = destinationEntityId,
    destinationDepth = destinationDepth,
    ancestorFolderIds = ancestorFolderIds,
    lowerOrder = lowerOrder,
    upperOrder = null,
  )
}
