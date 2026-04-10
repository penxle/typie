package co.typie.screen.auth.login

import co.typie.result.Result
import com.apollographql.apollo.ApolloClient
import com.apollographql.apollo.api.ApolloRequest
import com.apollographql.apollo.api.ApolloResponse
import com.apollographql.apollo.api.Operation
import com.apollographql.apollo.network.NetworkTransport
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class LoginViewModelTest {
  @Test
  fun `invalid email submission returns error without hitting network`() = runTest {
    val networkTransport = CountingNetworkTransport()
    val model =
      LoginWithEmailViewModel(
        apolloClient = ApolloClient.Builder().networkTransport(networkTransport).build()
      )

    model.state.form.email.setValue("invalid-email")
    model.state.form.password.setValue("password123")

    val result = model.submit()

    assertIs<Result.Err<LoginWithEmailError>>(result)
    assertEquals(0, networkTransport.requestCount)
    assertEquals(listOf("올바른 이메일 형식을 입력해주세요."), model.state.form.email.errors)
  }
}

private class CountingNetworkTransport : NetworkTransport {
  var requestCount = 0
    private set

  override fun <D : Operation.Data> execute(request: ApolloRequest<D>): Flow<ApolloResponse<D>> {
    requestCount += 1
    error("network should not be called when the login form is invalid")
  }

  override fun dispose() = Unit
}
