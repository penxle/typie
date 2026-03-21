package co.typie.form

fun interface Rule<V> {
  suspend fun validate(value: V): String?
}

fun <V> required(message: String = "필수 항목입니다"): Rule<V> = Rule { value ->
  when {
    value == null -> message
    value is String && value.isBlank() -> message
    else -> null
  }
}

fun email(message: String = "올바른 이메일 형식을 입력해주세요"): Rule<String> = Rule {
  if (it.isNotBlank() && !it.matches(Regex("^[A-Za-z0-9+_.-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,}$"))) message else null
}

fun minLength(min: Int, message: String = "${min}자 이상 입력해주세요"): Rule<String> = Rule {
  if (it.isNotBlank() && it.length < min) message else null
}

fun maxLength(max: Int, message: String = "${max}자 이하로 입력해주세요"): Rule<String> = Rule {
  if (it.length > max) message else null
}

fun pattern(regex: Regex, message: String = "올바른 형식을 입력해주세요"): Rule<String> = Rule {
  if (it.isNotBlank() && !it.matches(regex)) message else null
}

fun <V : Comparable<V>> min(min: V, message: String = "최솟값은 ${min}입니다"): Rule<V> = Rule {
  if (it < min) message else null
}

fun <V : Comparable<V>> max(max: V, message: String = "최댓값은 ${max}입니다"): Rule<V> = Rule {
  if (it > max) message else null
}

fun <V> rule(validate: suspend (V) -> String?): Rule<V> = Rule { validate(it) }
