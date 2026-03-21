package co.typie.form

import kotlinx.coroutines.test.runTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class RuleTest {

  @Test
  fun requiredRejectsBlankString() = runTest {
    val rule = required<String>()
    assertEquals("필수 항목입니다", rule.validate(""))
    assertEquals("필수 항목입니다", rule.validate("   "))
  }

  @Test
  fun requiredAcceptsNonBlankString() = runTest {
    assertNull(required<String>().validate("hello"))
  }

  @Test
  fun requiredRejectsNull() = runTest {
    val rule = required<String?>()
    assertEquals("필수 항목입니다", rule.validate(null))
  }

  @Test
  fun requiredAcceptsNonNullNonString() = runTest {
    assertNull(required<Int>().validate(0))
    assertNull(required<Int>().validate(42))
  }

  @Test
  fun requiredCustomMessage() = runTest {
    assertEquals("입력 필요", required<String>("입력 필요").validate(""))
  }

  @Test
  fun emailRejectsInvalid() = runTest {
    assertEquals("올바른 이메일 형식을 입력해주세요", email().validate("notanemail"))
    assertEquals("올바른 이메일 형식을 입력해주세요", email().validate("@no-local.com"))
  }

  @Test
  fun emailAcceptsValid() = runTest {
    assertNull(email().validate("user@example.com"))
    assertNull(email().validate("user+tag@sub.domain.com"))
  }

  @Test
  fun emailSkipsBlank() = runTest {
    assertNull(email().validate(""))
  }

  @Test
  fun minLengthRejectsTooShort() = runTest {
    assertEquals("6자 이상 입력해주세요", minLength(6).validate("abc"))
  }

  @Test
  fun minLengthAcceptsExact() = runTest {
    assertNull(minLength(3).validate("abc"))
  }

  @Test
  fun minLengthSkipsBlank() = runTest {
    assertNull(minLength(6).validate(""))
  }

  @Test
  fun maxLengthRejectsTooLong() = runTest {
    assertEquals("3자 이하로 입력해주세요", maxLength(3).validate("abcd"))
  }

  @Test
  fun maxLengthAcceptsExact() = runTest {
    assertNull(maxLength(3).validate("abc"))
  }

  @Test
  fun patternRejectsNonMatching() = runTest {
    val digitOnly = pattern(Regex("^\\d+$"), "숫자만 입력")
    assertEquals("숫자만 입력", digitOnly.validate("abc"))
  }

  @Test
  fun patternAcceptsMatching() = runTest {
    val digitOnly = pattern(Regex("^\\d+$"), "숫자만 입력")
    assertNull(digitOnly.validate("123"))
  }

  @Test
  fun minComparableRejects() = runTest {
    assertEquals("최솟값은 10입니다", min(10).validate(5))
  }

  @Test
  fun minComparableAccepts() = runTest {
    assertNull(min(10).validate(10))
    assertNull(min(10).validate(15))
  }

  @Test
  fun maxComparableRejects() = runTest {
    assertEquals("최댓값은 100입니다", max(100).validate(101))
  }

  @Test
  fun maxComparableAccepts() = runTest {
    assertNull(max(100).validate(100))
    assertNull(max(100).validate(50))
  }

  @Test
  fun customRule() = runTest {
    val noSpaces = rule<String> { if (it.contains(" ")) "공백 불가" else null }
    assertEquals("공백 불가", noSpaces.validate("has space"))
    assertNull(noSpaces.validate("nospace"))
  }

  @Test
  fun customAsyncRule() = runTest {
    val asyncRule = rule<String> { value ->
      if (value == "taken") "이미 사용 중" else null
    }
    assertEquals("이미 사용 중", asyncRule.validate("taken"))
    assertNull(asyncRule.validate("available"))
  }
}
