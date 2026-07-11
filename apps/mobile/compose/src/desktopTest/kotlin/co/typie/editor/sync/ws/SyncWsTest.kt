package co.typie.editor.sync.ws

import kotlin.test.Test
import kotlin.test.assertNotSame
import kotlin.test.assertSame

class SyncWsTest {
  @Test
  fun retryDocumentEvictsRegistryEntrySoNextLookupCreatesFreshChannel() {
    val documentId = "SYNC-WS-RETRY-TEST"
    val first = SyncWs.channel(documentId)
    assertSame(first, SyncWs.channel(documentId))

    SyncWs.retryDocument(documentId)

    val second = SyncWs.channel(documentId)
    assertNotSame(first, second)

    SyncWs.retryDocument(documentId)
  }
}
