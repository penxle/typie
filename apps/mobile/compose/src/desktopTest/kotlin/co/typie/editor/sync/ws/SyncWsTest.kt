package co.typie.editor.sync.ws

import kotlin.test.Test
import kotlin.test.assertSame

class SyncWsTest {
  @Test
  fun retryDocumentKeepsTheSharedChannelIdentity() {
    val documentId = "SYNC-WS-RETRY-TEST"
    val first = SyncWs.channel(documentId)
    assertSame(first, SyncWs.channel(documentId))

    SyncWs.retryDocument(documentId)

    assertSame(first, SyncWs.channel(documentId))

    SyncWs.retryDocument(documentId)
  }
}
