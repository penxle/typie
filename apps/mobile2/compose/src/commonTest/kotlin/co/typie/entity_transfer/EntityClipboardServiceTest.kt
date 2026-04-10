package co.typie.entity_transfer

import co.typie.graphql.TypieError
import co.typie.overlay.Toast
import co.typie.overlay.ToastType
import co.typie.service.DEFAULT_SITE_REFRESH_DEBOUNCE_MS
import co.typie.service.SiteRefreshCoordinator
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class EntityClipboardServiceTest {

  @Test
  fun `copy paste keeps clipboard state and sends copy request`() = runTest {
    val executor = FakeEntityClipboardMutationExecutor()
    val toast = Toast()
    val coordinator = SiteRefreshCoordinator()
    val service = EntityClipboardService(executor, toast, coordinator)
    val target = rootTarget(siteId = "site-b")

    service.setCopy(
      sourceSiteId = "site-a",
      items = listOf(EntityTransferSource.Document(id = "document-1", title = "문서", depth = 2)),
    )

    assertTrue(service.pasteInto(target))

    assertNotNull(service.state)
    assertEquals(
      listOf(
        EntityClipboardCopyRequest(
          entityIds = listOf("document-1"),
          targetSiteId = "site-b",
          parentEntityId = null,
          lowerOrder = null,
          upperOrder = null,
        )
      ),
      executor.copyRequests,
    )
    assertTrue(executor.moveRequests.isEmpty())
    assertEquals(ToastType.Success, toast.state.value?.type)
    assertEquals("1개의 항목을 붙여넣었어요", toast.state.value?.message)
  }

  @Test
  fun `cut paste clears clipboard state and sends same-site move request`() = runTest {
    val executor = FakeEntityClipboardMutationExecutor()
    val toast = Toast()
    val coordinator = SiteRefreshCoordinator()
    val service = EntityClipboardService(executor, toast, coordinator)
    val target =
      folderTarget(
        siteId = "site-a",
        destinationEntityId = "folder-b",
        destinationDepth = 2,
        lowerOrder = "after-last",
      )

    service.setCut(
      sourceSiteId = "site-a",
      items = listOf(EntityTransferSource.Document(id = "document-1", title = "문서", depth = 1)),
    )

    assertTrue(service.pasteInto(target))

    assertNull(service.state)
    assertEquals(
      listOf(
        EntityClipboardMoveRequest(
          entityIds = listOf("document-1"),
          parentEntityId = "folder-b",
          lowerOrder = "after-last",
          upperOrder = null,
          targetSiteId = null,
        )
      ),
      executor.moveRequests,
    )
    assertEquals(ToastType.Success, toast.state.value?.type)
  }

  @Test
  fun `cross-site cut includes target site id and notifies source and target`() = runTest {
    val executor = FakeEntityClipboardMutationExecutor()
    val toast = Toast()
    val coordinator = SiteRefreshCoordinator()
    val service = EntityClipboardService(executor, toast, coordinator)
    val refreshes = mutableListOf<String>()
    val sourceJob = launch { coordinator.refreshes("site-a").collect { refreshes += "site-a" } }
    val targetJob = launch { coordinator.refreshes("site-b").collect { refreshes += "site-b" } }
    runCurrent()

    service.setCut(
      sourceSiteId = "site-a",
      items = listOf(EntityTransferSource.Document(id = "document-1", title = "문서", depth = 1)),
    )

    assertTrue(service.pasteInto(rootTarget(siteId = "site-b")))
    advanceTimeBy(DEFAULT_SITE_REFRESH_DEBOUNCE_MS)
    runCurrent()

    assertEquals("site-b", executor.moveRequests.single().targetSiteId)
    assertEquals(setOf("site-a", "site-b"), refreshes.toSet())

    sourceJob.cancel()
    targetJob.cancel()
  }

  @Test
  fun `cannot paste folder into itself or its descendants`() = runTest {
    val executor = FakeEntityClipboardMutationExecutor()
    val service = EntityClipboardService(executor, Toast(), SiteRefreshCoordinator())
    val source =
      EntityTransferSource.Folder(
        id = "folder-a",
        title = "폴더",
        depth = 3,
        maxDescendantFoldersDepth = 4,
      )

    service.setCut(sourceSiteId = "site-a", items = listOf(source))

    assertFalse(
      service.canPaste(
        folderTarget(siteId = "site-a", destinationEntityId = "folder-a", destinationDepth = 3)
      )
    )
    assertFalse(
      service.canPaste(
        folderTarget(
          siteId = "site-a",
          destinationEntityId = "folder-child",
          destinationDepth = 4,
          ancestorFolderIds = setOf("folder-a"),
        )
      )
    )
  }

  @Test
  fun `paste failure maps latest web error message`() = runTest {
    val executor =
      FakeEntityClipboardMutationExecutor(
        copyError = TypieError(code = "circular_reference", message = "cycle")
      )
    val toast = Toast()
    val service = EntityClipboardService(executor, toast, SiteRefreshCoordinator())

    service.setCopy(
      sourceSiteId = "site-a",
      items = listOf(EntityTransferSource.Document(id = "document-1", title = "문서", depth = 1)),
    )

    assertFalse(service.pasteInto(rootTarget(siteId = "site-a")))
    assertEquals(ToastType.Error, toast.state.value?.type)
    assertEquals("자기 자신 또는 하위 항목 안에는 붙여넣을 수 없어요.", toast.state.value?.message)
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
}

private class FakeEntityClipboardMutationExecutor(
  private val copyError: Throwable? = null,
  private val moveError: Throwable? = null,
) : EntityClipboardMutationExecutor {
  val copyRequests = mutableListOf<EntityClipboardCopyRequest>()
  val moveRequests = mutableListOf<EntityClipboardMoveRequest>()

  override suspend fun copyEntities(request: EntityClipboardCopyRequest) {
    copyRequests += request
    copyError?.let { throw it }
  }

  override suspend fun moveEntities(request: EntityClipboardMoveRequest) {
    moveRequests += request
    moveError?.let { throw it }
  }
}
