# Single Sign-On 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Google, Kakao, Naver, Apple 4개 SSO 로그인을 KMP expect/actual 패턴으로 구현한다.

**Architecture:** commonMain에 `SingleSignOnProvider` 인터페이스와 `LoginViewModel`을 두고, 각 플랫폼(Android/iOS/JVM)에서 네이티브 SDK로 actual 구현한다. LoginScreen에서 ViewModel을 통해 SSO 버튼을 연결한다. 모든 provider는 `PlatformContext`를 생성자로 받아 플랫폼 의존성을 해결한다.

**Tech Stack:** Credential Manager (Android Google), Kakao SDK v2, Naver SDK v5, AuthenticationServices (iOS Apple), Google Sign-In iOS SDK, Apollo GraphQL, Koin DI

**Spec:** `docs/superpowers/specs/2026-03-19-single-sign-on-design.md`

**Note:** 이 프로젝트는 `git commit`을 직접 수행하지 않는다. 커밋은 사용자가 수동으로 한다.

**Note:** 프로젝트에 JVM 데스크톱 타겟이 있으므로, `jvmMain`에도 actual stub이 필요하다. JVM에서는 SSO가 지원되지 않으므로 버튼을 숨긴다.

**Note:** Naver iOS SDK의 SPM product 이름은 `NaverThirdPartyLogin`이 아닌 `NidThirdPartyLogin`이다. 스펙과 다르므로 주의.

---

## 파일 구조

### 새로 생성

| 파일 | 역할 |
|------|------|
| `compose/src/commonMain/kotlin/co/typie/auth/sso/SingleSignOnCredential.kt` | 인증 결과 data class |
| `compose/src/commonMain/kotlin/co/typie/auth/sso/SingleSignOnProvider.kt` | provider 인터페이스 |
| `compose/src/commonMain/kotlin/co/typie/auth/sso/GoogleSingleSignOnProvider.kt` | expect class |
| `compose/src/commonMain/kotlin/co/typie/auth/sso/KakaoSingleSignOnProvider.kt` | expect class |
| `compose/src/commonMain/kotlin/co/typie/auth/sso/NaverSingleSignOnProvider.kt` | expect class |
| `compose/src/commonMain/kotlin/co/typie/auth/sso/AppleSingleSignOnProvider.kt` | expect class |
| `compose/src/commonMain/kotlin/co/typie/screen/login/AuthorizeSingleSignOn.graphql` | GraphQL mutation |
| `compose/src/commonMain/kotlin/co/typie/screen/login/LoginViewModel.kt` | SSO 로그인 ViewModel |
| `compose/src/androidMain/kotlin/co/typie/auth/sso/GoogleSingleSignOnProvider.kt` | actual (Credential Manager) |
| `compose/src/androidMain/kotlin/co/typie/auth/sso/KakaoSingleSignOnProvider.kt` | actual (Kakao SDK) |
| `compose/src/androidMain/kotlin/co/typie/auth/sso/NaverSingleSignOnProvider.kt` | actual (Naver SDK) |
| `compose/src/androidMain/kotlin/co/typie/auth/sso/AppleSingleSignOnProvider.kt` | actual (미지원 stub) |
| `compose/src/iosMain/kotlin/co/typie/auth/sso/GoogleSingleSignOnProvider.kt` | actual (GIDSignIn) |
| `compose/src/iosMain/kotlin/co/typie/auth/sso/KakaoSingleSignOnProvider.kt` | actual (KakaoSDK) |
| `compose/src/iosMain/kotlin/co/typie/auth/sso/NaverSingleSignOnProvider.kt` | actual (NidThirdPartyLogin) |
| `compose/src/iosMain/kotlin/co/typie/auth/sso/AppleSingleSignOnProvider.kt` | actual (AuthenticationServices) |
| `compose/src/jvmMain/kotlin/co/typie/auth/sso/GoogleSingleSignOnProvider.kt` | actual (미지원 stub) |
| `compose/src/jvmMain/kotlin/co/typie/auth/sso/KakaoSingleSignOnProvider.kt` | actual (미지원 stub) |
| `compose/src/jvmMain/kotlin/co/typie/auth/sso/NaverSingleSignOnProvider.kt` | actual (미지원 stub) |
| `compose/src/jvmMain/kotlin/co/typie/auth/sso/AppleSingleSignOnProvider.kt` | actual (미지원 stub) |

### 수정

| 파일 | 변경 내용 |
|------|----------|
| `compose/build.gradle.kts` | Android/iOS 의존성 추가, swiftPMDependencies 설정 |
| `settings.gradle.kts` | Kakao Maven 리포지토리 추가 |
| `gradle/libs.versions.toml` | 새 의존성 버전 및 라이브러리 추가 |
| `compose/src/commonMain/kotlin/co/typie/screen/login/LoginScreen.kt` | ViewModel 연결, 로딩/에러 상태, Platform 조건부 렌더링 |
| `android/src/main/kotlin/co/typie/MainApplication.kt` | Kakao/Naver SDK 초기화 |

---

### Task 1: 빌드 설정 — 의존성 추가

**Files:**
- Modify: `settings.gradle.kts`
- Modify: `gradle/libs.versions.toml`
- Modify: `compose/build.gradle.kts`

- [ ] **Step 1: `settings.gradle.kts`에 Kakao Maven 리포지토리 추가**

`dependencyResolutionManagement.repositories` 블록에서 기존 `mavenCentral()` 아래에 추가:

```kotlin
maven("https://devrepo.kakao.com/nexus/content/groups/public/")
```

- [ ] **Step 2: `gradle/libs.versions.toml`에 버전 및 라이브러리 추가**

`[versions]` 섹션에 추가 (알파벳 순):

```toml
androidx-credentials = "1.5.0"
googleid = "1.2.0"
kakao = "2.23.2"
naver-oauth = "5.11.2"
```

`[libraries]` 섹션에 추가 (알파벳 순):

```toml
androidx-credentials = { module = "androidx.credentials:credentials", version.ref = "androidx-credentials" }
androidx-credentials-playServicesAuth = { module = "androidx.credentials:credentials-play-services-auth", version.ref = "androidx-credentials" }
googleid = { module = "com.google.android.libraries.identity.googleid:googleid", version.ref = "googleid" }
kakao-user = { module = "com.kakao.sdk:v2-user", version.ref = "kakao" }
naver-oauth = { module = "com.navercorp.nid:oauth-jdk8", version.ref = "naver-oauth" }
```

- [ ] **Step 3: `compose/build.gradle.kts`에 Android 의존성 추가**

`sourceSets.androidMain.dependencies` 블록에 추가:

```kotlin
androidMain {
  dependencies {
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.credentials)
    implementation(libs.androidx.credentials.playServicesAuth)
    implementation(libs.googleid)
    implementation(libs.kakao.user)
    implementation(libs.naver.oauth)
  }
}
```

- [ ] **Step 4: `compose/build.gradle.kts`에 swiftPMDependencies 설정**

기존 `swiftPMDependencies` 블록을 업데이트 (`iosMinimumDeploymentTarget` 제거):

```kotlin
swiftPMDependencies {
  swiftPackage(
    url = url("https://github.com/google/GoogleSignIn-iOS.git"),
    version = from("9.0.0"),
    products = listOf(product("GoogleSignIn")),
  )
  swiftPackage(
    url = url("https://github.com/kakao/kakao-ios-sdk.git"),
    version = from("2.27.2"),
    products = listOf(product("KakaoSDKAuth"), product("KakaoSDKUser")),
  )
  swiftPackage(
    url = url("https://github.com/naver/naveridlogin-sdk-ios-swift.git"),
    version = from("5.1.0"),
    products = listOf(product("NidThirdPartyLogin")),
  )
}
```

- [ ] **Step 5: Gradle sync 확인**

Run: Gradle sync가 성공하는지 확인한다. 의존성 resolve 에러가 없어야 한다.

---

### Task 2: GraphQL mutation 파일 생성

**Files:**
- Create: `compose/src/commonMain/kotlin/co/typie/screen/login/AuthorizeSingleSignOn.graphql`

전제 조건: `compose/src/commonMain/graphql/schema.graphqls`에 `AuthorizeSingleSignOnInput`, `SingleSignOnProvider` enum, `authorizeSingleSignOn` mutation이 이미 정의되어 있어야 한다. 스키마가 없으면 서버에서 introspect하여 다운로드한다.

- [ ] **Step 1: mutation 파일 작성**

```graphql
mutation LoginScreen_AuthorizeSingleSignOn_Mutation($input: AuthorizeSingleSignOnInput!) {
  authorizeSingleSignOn(input: $input)
}
```

- [ ] **Step 2: Apollo 코드 생성 확인**

Run: `./gradlew :compose:generateApolloSources`

Expected: `LoginScreen_AuthorizeSingleSignOn_Mutation` 클래스가 생성된다.

---

### Task 3: commonMain 공통 타입 정의

**Files:**
- Create: `compose/src/commonMain/kotlin/co/typie/auth/sso/SingleSignOnCredential.kt`
- Create: `compose/src/commonMain/kotlin/co/typie/auth/sso/SingleSignOnProvider.kt`

- [ ] **Step 1: SingleSignOnCredential 작성**

```kotlin
package co.typie.auth.sso

import co.typie.graphql.type.SingleSignOnProvider as GraphQLSingleSignOnProvider

data class SingleSignOnCredential(
  val provider: GraphQLSingleSignOnProvider,
  val params: Map<String, String>,
)
```

- [ ] **Step 2: SingleSignOnProvider 인터페이스 작성**

```kotlin
package co.typie.auth.sso

interface SingleSignOnProvider {
  suspend fun authenticate(): SingleSignOnCredential
}
```

---

### Task 4: expect 클래스 선언 (commonMain)

**Files:**
- Create: `compose/src/commonMain/kotlin/co/typie/auth/sso/GoogleSingleSignOnProvider.kt`
- Create: `compose/src/commonMain/kotlin/co/typie/auth/sso/KakaoSingleSignOnProvider.kt`
- Create: `compose/src/commonMain/kotlin/co/typie/auth/sso/NaverSingleSignOnProvider.kt`
- Create: `compose/src/commonMain/kotlin/co/typie/auth/sso/AppleSingleSignOnProvider.kt`

모든 provider는 `PlatformContext`를 생성자로 받는다 (기존 `StorageModule` 패턴과 동일).

- [ ] **Step 1: 4개 expect 클래스 작성**

```kotlin
// GoogleSingleSignOnProvider.kt
package co.typie.auth.sso

import co.typie.di.PlatformContext
import org.koin.core.annotation.Single

@Single
expect class GoogleSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider
```

```kotlin
// KakaoSingleSignOnProvider.kt
package co.typie.auth.sso

import co.typie.di.PlatformContext
import org.koin.core.annotation.Single

@Single
expect class KakaoSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider
```

```kotlin
// NaverSingleSignOnProvider.kt
package co.typie.auth.sso

import co.typie.di.PlatformContext
import org.koin.core.annotation.Single

@Single
expect class NaverSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider
```

```kotlin
// AppleSingleSignOnProvider.kt
package co.typie.auth.sso

import co.typie.di.PlatformContext
import org.koin.core.annotation.Single

@Single
expect class AppleSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider
```

---

### Task 5: JVM actual stub (데스크톱 — 미지원)

**Files:**
- Create: `compose/src/jvmMain/kotlin/co/typie/auth/sso/GoogleSingleSignOnProvider.kt`
- Create: `compose/src/jvmMain/kotlin/co/typie/auth/sso/KakaoSingleSignOnProvider.kt`
- Create: `compose/src/jvmMain/kotlin/co/typie/auth/sso/NaverSingleSignOnProvider.kt`
- Create: `compose/src/jvmMain/kotlin/co/typie/auth/sso/AppleSingleSignOnProvider.kt`

- [ ] **Step 1: 4개 JVM actual 작성**

모든 JVM actual은 동일한 패턴 (SSO 미지원):

```kotlin
// GoogleSingleSignOnProvider.kt
package co.typie.auth.sso

import co.typie.di.PlatformContext

actual class GoogleSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider {
  override suspend fun authenticate(): SingleSignOnCredential {
    throw UnsupportedOperationException("Google SSO is not supported on JVM")
  }
}
```

KakaoSingleSignOnProvider, NaverSingleSignOnProvider, AppleSingleSignOnProvider도 동일하게 작성. 메시지만 provider 이름에 맞게 변경.

---

### Task 6: LoginViewModel 작성

**Files:**
- Create: `compose/src/commonMain/kotlin/co/typie/screen/login/LoginViewModel.kt`

참고: `LoginWithEmailViewModel.kt` 패턴을 따른다. GraphQL 에러 처리도 동일한 extensions 파싱 패턴 사용.

- [ ] **Step 1: LoginViewModel 작성**

```kotlin
package co.typie.screen.login

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import co.typie.auth.AuthStore
import co.typie.auth.sso.AppleSingleSignOnProvider
import co.typie.auth.sso.GoogleSingleSignOnProvider
import co.typie.auth.sso.KakaoSingleSignOnProvider
import co.typie.auth.sso.NaverSingleSignOnProvider
import co.typie.auth.sso.SingleSignOnCredential
import co.typie.graphql.type.AuthorizeSingleSignOnInput
import com.apollographql.apollo.ApolloClient
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import org.koin.core.annotation.KoinViewModel

@KoinViewModel
class LoginViewModel(
  private val googleProvider: GoogleSingleSignOnProvider,
  private val kakaoProvider: KakaoSingleSignOnProvider,
  private val naverProvider: NaverSingleSignOnProvider,
  private val appleProvider: AppleSingleSignOnProvider,
  private val apolloClient: ApolloClient,
  private val authStore: AuthStore,
) : ViewModel() {

  private val _loading = MutableStateFlow(false)
  val loading: StateFlow<Boolean> = _loading

  private val _error = MutableStateFlow<String?>(null)
  val error: StateFlow<String?> = _error

  fun loginWithGoogle() = loginWith(googleProvider)
  fun loginWithKakao() = loginWith(kakaoProvider)
  fun loginWithNaver() = loginWith(naverProvider)
  fun loginWithApple() = loginWith(appleProvider)

  private fun loginWith(provider: co.typie.auth.sso.SingleSignOnProvider) {
    viewModelScope.launch {
      _loading.value = true
      _error.value = null

      try {
        val credential = provider.authenticate()
        executeMutation(credential)
      } catch (e: CancellationException) {
        throw e // 구조화된 동시성 유지를 위해 rethrow
      } catch (e: Exception) {
        _error.value = "로그인에 실패했어요. 다시 시도해주세요."
      } finally {
        _loading.value = false
      }
    }
  }

  private suspend fun executeMutation(credential: SingleSignOnCredential) {
    val params = JsonObject(credential.params.mapValues { JsonPrimitive(it.value) })
    val input = AuthorizeSingleSignOnInput(
      provider = credential.provider,
      params = params,
    )

    val response = apolloClient
      .mutation(LoginScreen_AuthorizeSingleSignOn_Mutation(input))
      .execute()

    val gqlError = response.errors?.firstOrNull()
    if (gqlError != null) {
      val extensions = gqlError.extensions
      val code = (extensions?.get("code") as? String)

      _error.value = when (code) {
        else -> "로그인에 실패했어요. 다시 시도해주세요."
      }
      return
    }

    authStore.login()
  }
}
```

> Note: `AuthorizeSingleSignOnInput`의 `params` 필드는 GraphQL `JSON!` 타입이다. Apollo가 이를 어떤 Kotlin 타입으로 생성하는지 확인 필요. 프로젝트의 Apollo 스칼라 매핑에 따라 `Any`, `Map`, `JsonObject` 등이 될 수 있다. 위 코드는 `kotlinx.serialization.json.JsonObject`를 사용했으나, 실제 생성된 타입에 맞게 조정한다. SSO 관련 특수 에러 코드가 API에 존재하면 `when` 분기에 추가한다.

---

### Task 7: LoginScreen 수정

**Files:**
- Modify: `compose/src/commonMain/kotlin/co/typie/screen/login/LoginScreen.kt`

- [ ] **Step 1: ViewModel 연결 및 상태 처리**

LoginScreen에 다음 변경을 적용:

1. `LoginViewModel`을 `koinViewModel()`로 주입
2. `Platform`을 Koin에서 주입받아 SSO 버튼 조건부 렌더링 (JVM에서는 전체 숨김, iOS에서만 Apple 표시)
3. 각 `SsoButton`에 `onClick` 추가
4. 로딩 상태 시 버튼 비활성화
5. 에러 메시지 표시

```kotlin
@Composable
fun LoginScreen() {
  val viewModel = koinViewModel<LoginViewModel>()
  val loading by viewModel.loading.collectAsState()
  val error by viewModel.error.collectAsState()
  val platform = koinInject<Platform>()

  Screen {
    Column(
      modifier = Modifier
        .fillMaxSize()
        .windowInsetsPadding(WindowInsets.safeDrawing),
    ) {
      // ... 기존 상단 로고/텍스트 영역 유지 ...

      Column(
        modifier = Modifier.padding(horizontal = 20.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
      ) {
        if (platform != Platform.Jvm) {
          SsoButton(
            text = "구글로 시작하기",
            svgPath = "files/brands/google.svg",
            foregroundColor = Color(0xFF000000),
            backgroundColor = Color(0xFFFFFFFF),
            borderColor = AppTheme.colors.borderDefault,
            enabled = !loading,
            onClick = { viewModel.loginWithGoogle() },
          )
          SsoButton(
            text = "카카오로 시작하기",
            svgPath = "files/brands/kakao.svg",
            iconTint = Color(0xFF000000),
            foregroundColor = Color(0xFF000000),
            backgroundColor = Color(0xFFFEE500),
            enabled = !loading,
            onClick = { viewModel.loginWithKakao() },
          )
          SsoButton(
            text = "네이버로 시작하기",
            svgPath = "files/brands/naver.svg",
            iconTint = Color(0xFFFFFFFF),
            foregroundColor = Color(0xFFFFFFFF),
            backgroundColor = Color(0xFF03C75A),
            enabled = !loading,
            onClick = { viewModel.loginWithNaver() },
          )

          if (platform == Platform.iOS) {
            SsoButton(
              text = "애플로 시작하기",
              svgPath = "files/brands/apple.svg",
              iconTint = Color(0xFFFFFFFF),
              foregroundColor = Color(0xFFFFFFFF),
              backgroundColor = Color(0xFF000000),
              enabled = !loading,
              onClick = { viewModel.loginWithApple() },
            )
          }
        }

        if (error != null) {
          Text(
            error!!,
            style = TextStyle(fontSize = 13.sp, color = AppTheme.colors.textDanger),
            modifier = Modifier.padding(top = 4.dp),
          )
        }

        Text(
          "이메일로 가입하셨나요?",
          style = TextStyle(fontSize = 14.sp, color = AppTheme.colors.textSubtle),
          modifier = Modifier
            .padding(horizontal = 24.dp, vertical = 8.dp)
            .clickable { Nav.current.navigate(Route.LoginWithEmail) },
        )
      }
    }
  }
}
```

- [ ] **Step 2: SsoButton에 `enabled`와 `onClick` 파라미터 추가**

```kotlin
@Composable
private fun SsoButton(
  text: String,
  svgPath: String,
  foregroundColor: Color,
  backgroundColor: Color,
  borderColor: Color? = null,
  iconTint: Color? = null,
  enabled: Boolean = true,
  onClick: () -> Unit = {},
) {
  val shape = RoundedCornerShape(999.dp)
  val alpha = if (enabled) 1f else 0.5f

  Box(
    modifier = Modifier
      .fillMaxWidth()
      .height(48.dp)
      .alpha(alpha)
      .then(if (borderColor != null) Modifier.border(1.dp, borderColor, shape) else Modifier)
      .background(backgroundColor, shape)
      .then(if (enabled) Modifier.clickable(onClick = onClick) else Modifier),
  ) {
    // ... 기존 AsyncImage + Text 유지 ...
  }
}
```

import 추가: `import androidx.compose.ui.draw.alpha`, `import co.typie.di.Platform`, `import org.koin.compose.koinInject`

---

### Task 8: Android actual — Google

**Files:**
- Create: `compose/src/androidMain/kotlin/co/typie/auth/sso/GoogleSingleSignOnProvider.kt`

참고: Android Credential Manager + `GetGoogleIdOption` 사용. `setFilterByAuthorizedAccounts(false)`로 항상 계정 선택 표시.

- [ ] **Step 1: Android Google actual 작성**

```kotlin
package co.typie.auth.sso

import androidx.credentials.CredentialManager
import androidx.credentials.GetCredentialRequest
import co.typie.BuildKonfig
import co.typie.di.PlatformContext
import co.typie.graphql.type.SingleSignOnProvider
import com.google.android.libraries.identity.googleid.GetGoogleIdOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class GoogleSingleSignOnProvider(
  private val ctx: PlatformContext,
) : SingleSignOnProvider {

  override suspend fun authenticate(): SingleSignOnCredential {
    val googleIdOption = GetGoogleIdOption.Builder()
      .setServerClientId(BuildKonfig.GOOGLE_CLIENT_ID)
      .setFilterByAuthorizedAccounts(false)
      .build()

    val request = GetCredentialRequest.Builder()
      .addCredentialOption(googleIdOption)
      .build()

    val credentialManager = CredentialManager.create(ctx.context)
    val result = credentialManager.getCredential(ctx.context, request)
    val googleIdTokenCredential = GoogleIdTokenCredential.createFrom(result.credential.data)

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.GOOGLE,
      params = mapOf("code" to googleIdTokenCredential.idToken),
    )
  }
}
```

> Note: `BuildKonfig.GOOGLE_CLIENT_ID`가 필요하다. 기존 Flutter 앱의 Google OAuth client ID를 참조하여 `build.gradle.kts`의 `buildkonfig` 블록에 추가해야 한다. Credential Manager의 `getCredential()`은 Activity context가 필요할 수 있다 — 실제 테스트 시 Application context로 동작하지 않으면 Activity context를 전달하도록 조정한다.

---

### Task 9: Android actual — Kakao

**Files:**
- Create: `compose/src/androidMain/kotlin/co/typie/auth/sso/KakaoSingleSignOnProvider.kt`

참고: `loginWithKakaoAccount(prompts = listOf(Prompt.LOGIN))`으로 항상 재인증.

- [ ] **Step 1: Android Kakao actual 작성**

```kotlin
package co.typie.auth.sso

import co.typie.di.PlatformContext
import co.typie.graphql.type.SingleSignOnProvider
import com.kakao.sdk.auth.model.Prompt
import com.kakao.sdk.user.UserApiClient
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class KakaoSingleSignOnProvider(
  private val ctx: PlatformContext,
) : SingleSignOnProvider {

  override suspend fun authenticate(): SingleSignOnCredential {
    val token = suspendCancellableCoroutine { continuation ->
      UserApiClient.instance.loginWithKakaoAccount(
        context = ctx.context,
        prompts = listOf(Prompt.LOGIN),
      ) { token, error ->
        if (error != null) {
          continuation.resumeWithException(error)
        } else if (token != null) {
          continuation.resume(token)
        } else {
          continuation.resumeWithException(IllegalStateException("No token received"))
        }
      }
    }

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.KAKAO,
      params = mapOf("access_token" to token.accessToken),
    )
  }
}
```

> Note: Kakao SDK 초기화(`KakaoSdk.init()`)가 Application 클래스에서 필요하다. Task 16에서 처리.

---

### Task 10: Android actual — Naver

**Files:**
- Create: `compose/src/androidMain/kotlin/co/typie/auth/sso/NaverSingleSignOnProvider.kt`

참고: SDK 한계로 `logout()` 후 `authenticate()` 호출 필요.

- [ ] **Step 1: Android Naver actual 작성**

```kotlin
package co.typie.auth.sso

import co.typie.di.PlatformContext
import co.typie.graphql.type.SingleSignOnProvider
import com.navercorp.nid.NaverIdLoginSDK
import com.navercorp.nid.oauth.OAuthLoginCallback
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class NaverSingleSignOnProvider(
  private val ctx: PlatformContext,
) : SingleSignOnProvider {

  override suspend fun authenticate(): SingleSignOnCredential {
    NaverIdLoginSDK.logout()

    val accessToken = suspendCancellableCoroutine { continuation ->
      NaverIdLoginSDK.authenticate(ctx.context, object : OAuthLoginCallback {
        override fun onSuccess() {
          val token = NaverIdLoginSDK.getAccessToken()
          if (token != null) {
            continuation.resume(token)
          } else {
            continuation.resumeWithException(IllegalStateException("No access token"))
          }
        }

        override fun onFailure(httpStatus: Int, message: String) {
          continuation.resumeWithException(RuntimeException("Naver login failed: $message"))
        }

        override fun onError(errorCode: Int, message: String) {
          continuation.resumeWithException(RuntimeException("Naver login error: $message"))
        }
      })
    }

    return SingleSignOnCredential(
      provider = SingleSignOnProvider.NAVER,
      params = mapOf("access_token" to accessToken),
    )
  }
}
```

> Note: Naver SDK 초기화(`NaverIdLoginSDK.initialize()`)가 Application 클래스에서 필요하다. Task 16에서 처리.

---

### Task 11: Android actual — Apple (미지원 stub)

**Files:**
- Create: `compose/src/androidMain/kotlin/co/typie/auth/sso/AppleSingleSignOnProvider.kt`

- [ ] **Step 1: Android Apple stub 작성**

```kotlin
package co.typie.auth.sso

import co.typie.di.PlatformContext

actual class AppleSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider {
  override suspend fun authenticate(): SingleSignOnCredential {
    throw UnsupportedOperationException("Apple SSO is not supported on Android")
  }
}
```

---

### Task 12: iOS actual — Google

**Files:**
- Create: `compose/src/iosMain/kotlin/co/typie/auth/sso/GoogleSingleSignOnProvider.kt`

참고: `GIDSignIn.sharedInstance.signIn(withPresenting:)`은 Obj-C API로 cinterop 접근. delegate 객체는 GC 방지를 위해 클래스 프로퍼티로 유지.

- [ ] **Step 1: iOS Google actual 작성**

```kotlin
package co.typie.auth.sso

import co.typie.di.PlatformContext
import co.typie.graphql.type.SingleSignOnProvider
import kotlinx.coroutines.suspendCancellableCoroutine
import platform.UIKit.UIApplication
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class GoogleSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider {

  override suspend fun authenticate(): SingleSignOnCredential {
    return suspendCancellableCoroutine { continuation ->
      val rootViewController = UIApplication.sharedApplication.keyWindow?.rootViewController
        ?: run {
          continuation.resumeWithException(IllegalStateException("No root view controller"))
          return@suspendCancellableCoroutine
        }

      GIDSignIn.sharedInstance.signInWithPresentingViewController(rootViewController) { result, error ->
        if (error != null) {
          continuation.resumeWithException(Exception(error.localizedDescription))
          return@signInWithPresentingViewController
        }

        val serverAuthCode = result?.serverAuthCode
        if (serverAuthCode != null) {
          continuation.resume(
            SingleSignOnCredential(
              provider = SingleSignOnProvider.GOOGLE,
              params = mapOf("code" to serverAuthCode),
            )
          )
        } else {
          continuation.resumeWithException(IllegalStateException("No server auth code"))
        }
      }
    }
  }
}
```

> Note: 실제 import 경로는 swiftPMDependencies가 생성하는 cinterop 모듈명에 따라 달라진다. `import swiftPMImport.*.GIDSignIn` 형태가 될 수 있다. Gradle sync 후 자동완성으로 확인한다. `GIDConfiguration`에 client ID 설정도 필요하다 — iOS 앱의 `Info.plist`에서 URL scheme과 함께 설정해야 한다.

---

### Task 13: iOS actual — Kakao

**Files:**
- Create: `compose/src/iosMain/kotlin/co/typie/auth/sso/KakaoSingleSignOnProvider.kt`

- [ ] **Step 1: iOS Kakao actual 작성**

```kotlin
package co.typie.auth.sso

import co.typie.di.PlatformContext
import co.typie.graphql.type.SingleSignOnProvider
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class KakaoSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider {

  override suspend fun authenticate(): SingleSignOnCredential {
    return suspendCancellableCoroutine { continuation ->
      UserApi.shared.loginWithKakaoAccount(prompts: listOf(Prompt.login)) { token, error ->
        if (error != null) {
          continuation.resumeWithException(Exception(error.localizedDescription))
          return@loginWithKakaoAccount
        }

        val accessToken = token?.accessToken
        if (accessToken != null) {
          continuation.resume(
            SingleSignOnCredential(
              provider = SingleSignOnProvider.KAKAO,
              params = mapOf("access_token" to accessToken),
            )
          )
        } else {
          continuation.resumeWithException(IllegalStateException("No access token"))
        }
      }
    }
  }
}
```

> Note: Kakao iOS SDK의 Obj-C 인터페이스가 swiftPMDependencies cinterop으로 노출되는 방식을 확인해야 한다. Swift-only API는 cinterop으로 접근 불가할 수 있다. 필요 시 Swift 브릿지 코드를 작성해야 할 수 있다. Kakao SDK 초기화도 iOS 앱 시작 시 필요하다.

---

### Task 14: iOS actual — Naver

**Files:**
- Create: `compose/src/iosMain/kotlin/co/typie/auth/sso/NaverSingleSignOnProvider.kt`

참고: Naver iOS SDK는 delegate 패턴 사용. Kotlin/Native에서 `NSObject()` + protocol 채택으로 구현. delegate 객체를 클래스 프로퍼티로 유지하여 GC 방지.

- [ ] **Step 1: iOS Naver actual 작성**

```kotlin
package co.typie.auth.sso

import co.typie.di.PlatformContext
import co.typie.graphql.type.SingleSignOnProvider
import kotlinx.coroutines.suspendCancellableCoroutine
import platform.Foundation.NSObject
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class NaverSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider {

  private var currentDelegate: NSObject? = null

  override suspend fun authenticate(): SingleSignOnCredential {
    return suspendCancellableCoroutine { continuation ->
      val connection = NidThirdPartyLoginConnection.getSharedInstance()

      val delegate = object : NSObject(), NidThirdPartyLoginConnectionDelegateProtocol {
        override fun oauth20ConnectionDidFinishRequestACTokenWithAuthCode() {
          val accessToken = connection.accessToken
          if (accessToken != null) {
            continuation.resume(
              SingleSignOnCredential(
                provider = SingleSignOnProvider.NAVER,
                params = mapOf("access_token" to accessToken),
              )
            )
          } else {
            continuation.resumeWithException(IllegalStateException("No access token"))
          }
          currentDelegate = null
        }

        override fun oauth20ConnectionDidFinishRequestACTokenWithRefreshToken() {
          // refresh — 로그인 플로우에서는 사용되지 않음
          currentDelegate = null
        }

        override fun oauth20ConnectionDidFinishDeleteToken() {
          // 토큰 삭제 — 로그인 플로우에서는 사용되지 않음
          currentDelegate = null
        }

        override fun oauth20Connection(oauthConnection: Any?, didFailWithError: NSError) {
          continuation.resumeWithException(Exception(didFailWithError.localizedDescription))
          currentDelegate = null
        }
      }

      currentDelegate = delegate
      connection.delegate = delegate
      connection.requestThirdPartyLogin()

      continuation.invokeOnCancellation {
        currentDelegate = null
      }
    }
  }
}
```

> Note: `NidThirdPartyLoginConnection`, `NidThirdPartyLoginConnectionDelegateProtocol` 등의 실제 클래스/프로토콜 이름은 cinterop 매핑에 따라 달라진다. Gradle sync 후 자동완성으로 확인한다. SDK 초기화 (client ID, URL scheme 등)도 iOS 앱 시작 시 필요하다.

---

### Task 15: iOS actual — Apple

**Files:**
- Create: `compose/src/iosMain/kotlin/co/typie/auth/sso/AppleSingleSignOnProvider.kt`

참고: `AuthenticationServices`는 iOS 시스템 프레임워크이므로 별도 SPM 패키지 불필요. delegate 객체를 클래스 프로퍼티로 유지하여 GC 방지.

- [ ] **Step 1: iOS Apple actual 작성**

```kotlin
package co.typie.auth.sso

import co.typie.di.PlatformContext
import co.typie.graphql.type.SingleSignOnProvider
import kotlinx.coroutines.suspendCancellableCoroutine
import platform.AuthenticationServices.*
import platform.Foundation.NSData
import platform.Foundation.NSError
import platform.Foundation.NSString
import platform.Foundation.NSUTF8StringEncoding
import platform.Foundation.create
import platform.UIKit.UIApplication
import platform.UIKit.UIWindow
import platform.darwin.NSObject
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

actual class AppleSingleSignOnProvider(ctx: PlatformContext) : SingleSignOnProvider {

  private var currentDelegate: NSObject? = null

  override suspend fun authenticate(): SingleSignOnCredential {
    return suspendCancellableCoroutine { continuation ->
      val provider = ASAuthorizationAppleIDProvider()
      val request = provider.createRequest()
      request.requestedScopes = listOf(ASAuthorizationScopeEmail)

      val controller = ASAuthorizationController(authorizationRequests = listOf(request))

      val delegate = object : NSObject(), ASAuthorizationControllerDelegateProtocol,
        ASAuthorizationControllerPresentationContextProvidingProtocol {

        override fun authorizationController(
          controller: ASAuthorizationController,
          didCompleteWithAuthorization: ASAuthorization,
        ) {
          val credential = didCompleteWithAuthorization.credential as? ASAuthorizationAppleIDCredential
          val codeData = credential?.authorizationCode
          val authorizationCode = codeData?.let {
            NSString.create(data = it, encoding = NSUTF8StringEncoding)?.toString()
          }

          if (authorizationCode != null) {
            continuation.resume(
              SingleSignOnCredential(
                provider = SingleSignOnProvider.APPLE,
                params = mapOf("code" to authorizationCode),
              )
            )
          } else {
            continuation.resumeWithException(IllegalStateException("No authorization code"))
          }
          currentDelegate = null
        }

        override fun authorizationController(
          controller: ASAuthorizationController,
          didCompleteWithError: NSError,
        ) {
          continuation.resumeWithException(Exception(didCompleteWithError.localizedDescription))
          currentDelegate = null
        }

        override fun presentationAnchorForAuthorizationController(
          controller: ASAuthorizationController,
        ): UIWindow {
          return UIApplication.sharedApplication.keyWindow!!
        }
      }

      currentDelegate = delegate
      controller.delegate = delegate
      controller.presentationContextProvider = delegate
      controller.performRequests()

      continuation.invokeOnCancellation {
        currentDelegate = null
      }
    }
  }
}
```

> Note: `ASAuthorizationControllerDelegateProtocol` 등의 실제 프로토콜 이름은 Kotlin/Native cinterop 매핑에 따라 달라질 수 있다. `authorizationCode`는 `NSData`이므로 `NSString`으로 변환한다. Xcode 프로젝트에서 "Sign in with Apple" capability를 활성화해야 한다.

---

### Task 16: 플랫폼 SDK 초기화

**Files:**
- Modify: `android/src/main/kotlin/co/typie/MainApplication.kt`
- Modify: `ios/typie/iOSApp.swift` (또는 해당 Swift 파일)

- [ ] **Step 1: Android MainApplication에 SDK 초기화 추가**

기존 `MainApplication.kt`의 `initKoin {}` 호출 이후에 SDK 초기화를 추가한다. 기존 `initKoin` 구조를 보존한다:

```kotlin
import com.kakao.sdk.common.KakaoSdk
import com.navercorp.nid.NaverIdLoginSDK

override fun onCreate() {
  super.onCreate()

  initKoin {
    androidContext(this@MainApplication)
    androidLogger()
  }

  KakaoSdk.init(this, BuildKonfig.KAKAO_NATIVE_APP_KEY)
  NaverIdLoginSDK.initialize(
    this,
    BuildKonfig.NAVER_CLIENT_ID,
    BuildKonfig.NAVER_CLIENT_SECRET,
    BuildKonfig.NAVER_CLIENT_NAME,
  )
}
```

- [ ] **Step 2: iOS 앱에 SDK 초기화 추가**

Google Sign-In의 URL scheme 처리, Kakao SDK 초기화, Naver SDK 초기화 등은 Swift 코드에서 설정해야 한다. `iOSApp.swift`의 `init()` 또는 `AppDelegate`에 추가한다.

- [ ] **Step 3: BuildKonfig에 키 값 추가**

`compose/build.gradle.kts`의 `buildkonfig` 블록에 필요한 상수를 추가한다. 실제 값은 기존 Flutter 앱의 설정을 참조한다:

- `GOOGLE_CLIENT_ID`
- `KAKAO_NATIVE_APP_KEY`
- `NAVER_CLIENT_ID`
- `NAVER_CLIENT_SECRET`
- `NAVER_CLIENT_NAME`

---

### Task 17: 통합 테스트

- [ ] **Step 1: 빌드 확인**

Run: `./gradlew :compose:build`

Expected: Android, iOS, JVM 모든 타겟이 컴파일 성공.

- [ ] **Step 2: Android 에뮬레이터에서 SSO 플로우 테스트**

각 SSO 버튼을 탭하여:
- 계정 선택 화면이 표시되는지 확인
- 인증 후 홈 화면으로 전환되는지 확인
- 취소 시 아무 동작 없이 로그인 화면에 머무르는지 확인

- [ ] **Step 3: iOS 시뮬레이터에서 SSO 플로우 테스트**

Android와 동일한 테스트. 추가로:
- Apple 버튼이 표시되는지 확인
- Apple Sign-In 시스템 다이얼로그가 표시되는지 확인

- [ ] **Step 4: JVM 데스크톱에서 SSO 버튼 미표시 확인**

JVM 타겟에서는 SSO 버튼이 표시되지 않고, LoginScreen 진입 시 크래시가 발생하지 않는지 확인한다.
