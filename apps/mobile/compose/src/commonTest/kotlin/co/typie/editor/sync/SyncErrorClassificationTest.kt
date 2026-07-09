package co.typie.editor.sync

import co.typie.graphql.TypieError
import com.apollographql.apollo.exception.ApolloHttpException
import com.apollographql.apollo.exception.ApolloNetworkException
import com.apollographql.apollo.exception.SubscriptionOperationException
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class SyncErrorClassificationTest {
  @Test
  fun invalidChangesetPayloadIsPermanent() {
    assertTrue(isPermanentSyncError(TypieError(code = "invalid_changeset_payload", message = "")))
  }

  @Test
  fun otherTypieErrorsAreTransient() {
    assertFalse(isPermanentSyncError(TypieError(code = "rate_limited", message = "")))
  }

  @Test
  fun http4xxIsPermanent() {
    assertTrue(
      isPermanentSyncError(
        ApolloHttpException(statusCode = 403, headers = emptyList(), body = null, message = "")
      )
    )
  }

  @Test
  fun http5xxAndNetworkErrorsAreTransient() {
    assertFalse(
      isPermanentSyncError(
        ApolloHttpException(statusCode = 502, headers = emptyList(), body = null, message = "")
      )
    )
    assertFalse(isPermanentSyncError(ApolloNetworkException("offline")))
  }

  @Test
  fun subscriptionOperationExceptionIsPermanent() {
    assertTrue(
      isPermanentSyncError(
        SubscriptionOperationException("DocumentSync_ChangesetsUpdated_Subscription", null)
      )
    )
  }

  @Test
  fun wrappedPermanentErrorsAreUnwrapped() {
    val causeWrapped =
      RuntimeException("wrapper", TypieError(code = "invalid_changeset_payload", message = ""))
    assertTrue(isPermanentSyncError(causeWrapped))

    val suppressedWrapped = RuntimeException("outer")
    suppressedWrapped.addSuppressed(
      ApolloHttpException(statusCode = 404, headers = emptyList(), body = null, message = "")
    )
    assertTrue(isPermanentSyncError(suppressedWrapped))

    val wrappedTransient = RuntimeException("wrapper", ApolloNetworkException("offline"))
    assertFalse(isPermanentSyncError(wrappedTransient))
  }

  @Test
  fun mutuallySuppressedCycleTerminates() {
    val a = RuntimeException("a")
    val b = RuntimeException("b")
    a.addSuppressed(b)
    b.addSuppressed(a)
    assertFalse(isPermanentSyncError(a))

    val c = RuntimeException("c")
    val d = RuntimeException("d", TypieError(code = "invalid_changeset_payload", message = ""))
    c.addSuppressed(d)
    d.addSuppressed(c)
    assertTrue(isPermanentSyncError(c))
  }
}
