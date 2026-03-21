package co.typie.form

import kotlinx.coroutines.delay
import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class TestLoginForm : FormState() {
  val email = field("") { required(); email() }
  val password = field("") { required(); minLength(6) }
}

class TestProfileForm(nickname: String, bio: String) : FormState() {
  val nickname = field(nickname) { required() }
  val bio = field(bio) { maxLength(500) }
}

class FormStateTest {

  @Test
  fun createForm() {
    val form = TestLoginForm()
    assertNotNull(form)
  }

  @Test
  fun fieldInitialValues() {
    val form = TestLoginForm()
    assertEquals("", form.email.value)
    assertEquals("", form.password.value)
  }

  @Test
  fun fieldValueReadWrite() {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    assertEquals("test@test.com", form.email.value)
  }

  @Test
  fun formIsDirtyWhenAnyFieldDirty() {
    val form = TestLoginForm()
    assertFalse(form.isDirty)
    form.email.setValue("changed")
    assertTrue(form.isDirty)
  }

  @Test
  fun formIsNotDirtyWhenAllFieldsClean() {
    val form = TestLoginForm()
    form.email.setValue("changed")
    form.email.setValue("")
    assertFalse(form.isDirty)
  }

  @Test
  fun formIsValidWhenNoErrors() {
    val form = TestLoginForm()
    assertTrue(form.isValid)
  }

  @Test
  fun formIsInvalidWhenFieldHasErrors() {
    val form = TestLoginForm()
    form.email.setErrors(listOf("에러"))
    assertFalse(form.isValid)
  }

  @Test
  fun formErrors() {
    val form = TestLoginForm()
    form.email.setErrors(listOf("이메일 에러"))
    form.password.setErrors(listOf("비밀번호 에러"))
    val errors = form.errors
    assertEquals(listOf("이메일 에러"), errors[form.email])
    assertEquals(listOf("비밀번호 에러"), errors[form.password])
  }

  @Test
  fun formReset() {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    form.password.setValue("secret")
    form.email.setErrors(listOf("에러"))

    form.reset()

    assertEquals("", form.email.value)
    assertEquals("", form.password.value)
    assertEquals(emptyList(), form.email.errors)
    assertFalse(form.isDirty)
  }

  @Test
  fun dynamicInitialValues() {
    val form = TestProfileForm(nickname = "기존닉네임", bio = "기존소개")
    assertEquals("기존닉네임", form.nickname.value)
    assertEquals("기존소개", form.bio.value)
  }

  @Test
  fun validateAllFieldsOnSubmit() = runTest {
    val form = TestLoginForm()
    val isValid = form.validateAll()
    assertFalse(isValid)
    assertEquals(listOf("필수 항목입니다"), form.email.errors)
    assertEquals(listOf("필수 항목입니다"), form.password.errors)
  }

  @Test
  fun validateAllPassesWhenValid() = runTest {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    form.password.setValue("secret123")
    val isValid = form.validateAll()
    assertTrue(isValid)
    assertEquals(emptyList(), form.email.errors)
    assertEquals(emptyList(), form.password.errors)
  }

  @Test
  fun defaultValidateOnPropagates() {
    class OnBlurForm : FormState(defaultValidateOn = ValidateOn.OnBlur) {
      val name = field("") { required() }
      val overridden = field("") { required(); validateOn(ValidateOn.OnChange) }
    }

    val form = OnBlurForm()
    assertEquals(ValidateOn.OnBlur, form.name.validateOn)
    assertEquals(ValidateOn.OnChange, form.overridden.validateOn)
  }

  @Test
  fun submitCallsBlockWhenValid() = runTest {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    form.password.setValue("secret123")

    var called = false
    form.submit(scope = this) {
      called = true
    }
    testScheduler.advanceUntilIdle()

    assertTrue(called)
  }

  @Test
  fun submitDoesNotCallBlockWhenInvalid() = runTest {
    val form = TestLoginForm()

    var called = false
    form.submit(scope = this) {
      called = true
    }
    testScheduler.advanceUntilIdle()

    assertFalse(called)
    assertEquals(listOf("필수 항목입니다"), form.email.errors)
  }

  @Test
  fun submitSetsIsSubmitting() = runTest {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    form.password.setValue("secret123")

    var wasSubmitting = false
    form.submit(scope = this) {
      wasSubmitting = form.isSubmitting
    }
    testScheduler.advanceUntilIdle()

    assertTrue(wasSubmitting)
    assertFalse(form.isSubmitting)
  }

  @Test
  fun submitPreventsDoubleSubmit() = runTest {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    form.password.setValue("secret123")

    var callCount = 0
    form.submit(scope = this) {
      callCount++
      delay(1000)
    }
    form.submit(scope = this) {
      callCount++
    }
    testScheduler.advanceUntilIdle()

    assertEquals(1, callCount)
  }

  @Test
  fun isProcessingDuringSubmit() = runTest {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    form.password.setValue("secret123")

    var wasProcessing = false
    form.submit(scope = this) {
      wasProcessing = form.isProcessing
    }
    testScheduler.advanceUntilIdle()

    assertTrue(wasProcessing)
    assertFalse(form.isProcessing)
  }

  @Test
  fun submitIsSubmittingFalseAfterException() = runTest {
    val form = TestLoginForm()
    form.email.setValue("test@test.com")
    form.password.setValue("secret123")

    var exceptionCaught = false
    form.submit(scope = this) {
      try {
        throw RuntimeException("서버 에러")
      } catch (_: RuntimeException) {
        exceptionCaught = true
      }
    }
    testScheduler.advanceUntilIdle()

    assertTrue(exceptionCaught)
    assertFalse(form.isSubmitting)
  }

  // --- Validation Timing Tests ---

  @Test
  fun onBlurTriggersValidation() = runTest {
    class BlurForm : FormState() {
      val email = field("") {
        required()
        validateOn(ValidateOn.OnBlur)
      }
    }

    val form = BlurForm()
    form.setValidationScope(this)

    form.email.onBlur()
    testScheduler.advanceUntilIdle()

    assertEquals(listOf("필수 항목입니다"), form.email.errors)
  }

  @Test
  fun onBlurWithoutScopeDoesNotCrash() {
    class BlurForm : FormState() {
      val email = field("") {
        required()
        validateOn(ValidateOn.OnBlur)
      }
    }

    val form = BlurForm()
    form.email.onBlur()
    assertEquals(emptyList(), form.email.errors)
  }

  @Test
  fun onBlurDoesNotTriggerForOnSubmitFields() = runTest {
    val form = TestLoginForm()
    form.setValidationScope(this)
    form.email.onBlur()
    testScheduler.advanceUntilIdle()

    assertEquals(emptyList(), form.email.errors)
  }

  @Test
  fun onChangeTriggersValidation() = runTest {
    class ChangeForm : FormState() {
      val email = field("") {
        email()
        validateOn(ValidateOn.OnChange)
      }
    }

    val form = ChangeForm()
    form.setValidationScope(this)

    form.email.setValue("invalid")
    testScheduler.advanceUntilIdle()

    assertEquals(listOf("올바른 이메일 형식을 입력해주세요"), form.email.errors)
  }

  @Test
  fun onSubmitAutoClearsErrors() {
    val form = TestLoginForm()
    form.email.setErrors(listOf("이전 에러"))
    form.email.setValue("new value")
    assertEquals(emptyList(), form.email.errors)
  }

  @Test
  fun onChangeDoesNotAutoClear() = runTest {
    class ChangeForm : FormState() {
      val email = field("") {
        required()
        validateOn(ValidateOn.OnChange)
      }
    }

    val form = ChangeForm()
    form.setValidationScope(this)

    form.email.setErrors(listOf("이전 에러"))
    form.email.setValue("new value")
    assertEquals(listOf("이전 에러"), form.email.errors)

    testScheduler.advanceUntilIdle()
    assertEquals(emptyList(), form.email.errors)
  }

  @Test
  fun onChangeDeferRulesDebounced() = runTest {
    var deferCallCount = 0
    val form = object : FormState() {
      val email = field("") {
        defer {
          deferCallCount++
          null
        }
        validateOn(ValidateOn.OnChange)
      }
    }
    form.setValidationScope(this)

    form.email.setValue("a")
    form.email.setValue("ab")
    form.email.setValue("abc")
    testScheduler.advanceUntilIdle()

    assertEquals(1, deferCallCount)
  }

  @Test
  fun onChangeSyncAndDeferCombined() = runTest {
    var deferCallCount = 0
    val form = object : FormState() {
      val email = field("") {
        email()
        defer {
          deferCallCount++
          if (it == "taken@test.com") "이미 사용 중" else null
        }
        validateOn(ValidateOn.OnChange)
      }
    }
    form.setValidationScope(this)

    form.email.setValue("invalid")
    testScheduler.advanceUntilIdle()
    assertEquals(listOf("올바른 이메일 형식을 입력해주세요"), form.email.errors)

    form.email.setValue("taken@test.com")
    testScheduler.advanceUntilIdle()
    assertEquals(listOf("이미 사용 중"), form.email.errors)
  }
}
