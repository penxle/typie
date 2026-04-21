package co.typie.screen.auth.login

import co.typie.result.Result
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.runTest

@OptIn(ExperimentalCoroutinesApi::class)
class LoginViewModelTest {
  @Test
  fun `invalid email submission returns error without hitting network`() = runTest {
    val model = LoginWithEmailViewModel()

    model.form.email.setValue("invalid-email")
    model.form.password.setValue("password123")

    val result = model.submit()

    assertIs<Result.Err<LoginWithEmailError>>(result)
    assertEquals(listOf("올바른 이메일 형식을 입력해주세요."), model.form.email.errors)
  }
}
