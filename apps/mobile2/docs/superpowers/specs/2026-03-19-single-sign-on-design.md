# Single Sign-On (SSO) 구현 설계

## 개요

KMP 프로젝트에 Google, Kakao, Naver, Apple 4개 SSO 로그인을 구현한다. commonMain에 공통 인터페이스를 정의하고, 각 플랫폼(Android/iOS)에서 네이티브 SDK로 actual 구현한다.

## 범위

- 4개 SSO provider 인증 플로우 구현
- Apple 로그인은 iOS에서만 표시
- 분석 이벤트(Mixpanel/AppsFlyer) 제외
- 토큰 저장소 연동 제외 (별도 구현 중)

## 아키텍처

### 핵심 타입 (commonMain)

```kotlin
// 인증 결과
data class SingleSignOnCredential(
  val provider: SingleSignOnProvider,  // GraphQL enum: GOOGLE, KAKAO, NAVER, APPLE
  val params: Map<String, String>,     // {"code": "..."} 또는 {"access_token": "..."}
)

// 각 provider가 구현하는 인터페이스
interface SingleSignOnProvider {
  suspend fun authenticate(): SingleSignOnCredential  // 취소 시 CancellationException
}
```

> 참고: `SingleSignOnProvider` 인터페이스와 GraphQL enum `SingleSignOnProvider`는 이름이 같지만 패키지로 구분된다. 혼동이 생기면 인터페이스 이름을 조정할 수 있다.

### Provider별 expect/actual 클래스

| 클래스 | Android actual | iOS actual |
|--------|---------------|------------|
| `GoogleSingleSignOnProvider` | Credential Manager | Google Sign-In iOS SDK (SPM) |
| `KakaoSingleSignOnProvider` | Kakao SDK v2 (`com.kakao.sdk:v2-user`) | Kakao iOS SDK (swiftPMDependencies) |
| `NaverSingleSignOnProvider` | Naver SDK v5 (`com.navercorp.nid:oauth-jdk8`) | Naver iOS SDK (swiftPMDependencies) |
| `AppleSingleSignOnProvider` | 미지원 (사용되지 않음) | AuthenticationServices (시스템 프레임워크) |

각 provider는 `@Single`로 Koin에 등록한다.

### LoginViewModel

`@KoinViewModel`으로 등록. 기존 `LoginWithEmailViewModel` 패턴을 따른다.

```
사용자가 SSO 버튼 탭
  → LoginViewModel.loginWith(provider)
  → SingleSignOnProvider.authenticate()  // 플랫폼 SDK 호출, 토큰/코드 획득
  → Apollo mutation: authorizeSingleSignOn(input)
  → 성공 시 authStore.login()
  → 실패 시 에러 메시지 표시
```

주입받는 의존성:
- `GoogleSingleSignOnProvider`
- `KakaoSingleSignOnProvider`
- `NaverSingleSignOnProvider`
- `AppleSingleSignOnProvider` (iOS에서만 사용)
- `ApolloClient`
- `AuthStore`

### 플랫폼별 Apple 버튼 표시

기존 `PlatformModule`이 주입하는 `Platform`을 사용한다. LoginScreen에서 `Platform`이 iOS일 때만 Apple 버튼을 렌더링한다.

### LoginScreen 변경

현재 비활성 상태인 SSO 버튼에 `LoginViewModel`의 함수를 연결한다. 로딩/에러 상태를 ViewModel에서 관리하고 UI에 반영한다.

## 플랫폼별 의존성

### Android (build.gradle.kts)

```kotlin
androidMain {
  dependencies {
    implementation("androidx.credentials:credentials:1.5.0")
    implementation("androidx.credentials:credentials-play-services-auth:1.5.0")
    implementation("com.google.android.libraries.identity.googleid:googleid:1.2.0")
    implementation("com.kakao.sdk:v2-user:2.23.2")
    implementation("com.navercorp.nid:oauth-jdk8:5.11.2")
  }
}
```

Kakao SDK는 별도 Maven 리포지토리 필요:
```kotlin
// settings.gradle.kts dependencyResolutionManagement.repositories
maven("https://devrepo.kakao.com/nexus/content/groups/public/")
```

### iOS (swiftPMDependencies)

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
    products = listOf(product("NaverThirdPartyLogin")),
  )
  // Apple Sign-In: AuthenticationServices는 시스템 프레임워크이므로 별도 패키지 불필요
}
```

## 파일 구조

```
compose/src/
  commonMain/kotlin/co/typie/
    auth/sso/
      SingleSignOnCredential.kt
      SingleSignOnProvider.kt          # interface
      GoogleSingleSignOnProvider.kt    # expect
      KakaoSingleSignOnProvider.kt     # expect
      NaverSingleSignOnProvider.kt     # expect
      AppleSingleSignOnProvider.kt     # expect
    screen/login/
      AuthorizeSingleSignOn.graphql    # mutation 파일
      LoginViewModel.kt
      LoginScreen.kt                   # 기존 파일 수정
  androidMain/kotlin/co/typie/
    auth/sso/
      GoogleSingleSignOnProvider.kt    # actual
      KakaoSingleSignOnProvider.kt     # actual
      NaverSingleSignOnProvider.kt     # actual
      AppleSingleSignOnProvider.kt     # actual (미지원)
  iosMain/kotlin/co/typie/
    auth/sso/
      GoogleSingleSignOnProvider.kt    # actual
      KakaoSingleSignOnProvider.kt     # actual
      NaverSingleSignOnProvider.kt     # actual
      AppleSingleSignOnProvider.kt     # actual
```

## GraphQL Mutation

```graphql
mutation LoginScreen_AuthorizeSingleSignOn_Mutation($input: AuthorizeSingleSignOnInput!) {
  authorizeSingleSignOn(input: $input)
}
```

Flutter 앱과 동일한 mutation 시그니처를 사용한다.

## 계정 선택 화면 보장

각 SDK가 매 로그인 시 계정 선택 화면을 항상 표시하도록 구현한다. Flutter 앱의 "logout → delay → login" 패턴은 사용하지 않고, 각 SDK의 공식 파라미터를 사용한다.

| Provider | 방법 | 사전 logout |
|----------|------|------------|
| **Google (Android)** | `GetGoogleIdOption.Builder().setFilterByAuthorizedAccounts(false)` → 항상 계정 선택 bottom sheet 표시 | 불필요 |
| **Google (iOS)** | `GIDSignIn.sharedInstance.signIn(withPresenting:)` → 매 호출마다 OAuth 플로우 표시 | 불필요 |
| **Kakao (양 플랫폼)** | `loginWithKakaoAccount(prompts: [Prompt.LOGIN])` → 기존 세션 무시, 재인증 강제 | 불필요 |
| **Naver (Android)** | `NaverIdLoginSDK.logout()` 후 `authenticate()` → 로컬 토큰 클리어 후 재인증 (SDK 한계로 사전 logout 필요) | 필요 |
| **Naver (iOS)** | SDK가 웹뷰로 로그인 플로우 표시 | 불필요 |
| **Apple (iOS)** | `ASAuthorizationAppleIDProvider().createRequest()` → 항상 시스템 다이얼로그 표시 | 불필요 |

## 에러 처리

- 사용자 취소: `CancellationException` → 무시 (아무 동작 안 함)
- SDK 에러: 일반 에러 메시지 표시 ("로그인에 실패했어요. 다시 시도해주세요.")
- GraphQL 에러: `LoginWithEmailViewModel`과 동일한 패턴으로 extensions에서 에러 코드 추출

## 플랫폼 설정 (앱 설정)

각 SDK는 앱 수준 설정이 필요하다 (client ID, URL scheme 등). 이는 기존 Flutter 앱의 설정을 참조한다:

- **Google**: OAuth client ID (Android: `credentials.xml`, iOS: `Info.plist` URL scheme)
- **Kakao**: Native app key (Android: `AndroidManifest.xml`, iOS: `Info.plist` URL scheme)
- **Naver**: Client ID/Secret (Android: `NaverIdLoginSDK.initialize()`, iOS: `Info.plist`)
- **Apple**: Capabilities에서 Sign in with Apple 활성화 (Xcode)
