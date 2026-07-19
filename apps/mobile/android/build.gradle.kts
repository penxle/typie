import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

val versionProps = Properties().apply { load(rootProject.file("version.properties").reader()) }
val debugKeystore = file("keystore-debug.jks")

val doppler: String =
  listOf("/opt/homebrew/bin/doppler", "/usr/local/bin/doppler").firstOrNull { file(it).exists() }
    ?: "doppler"

val dopplerSecrets: Map<String, String> by lazy {
  val output =
    providers
      .exec {
        commandLine(
          doppler,
          "secrets",
          "download",
          "-p",
          "mobile",
          "-c",
          "dev",
          "--no-file",
          "--format",
          "json",
        )
      }
      .standardOutput
      .asText
      .get()
  @Suppress("UNCHECKED_CAST")
  groovy.json.JsonSlurper().parseText(output) as Map<String, String>
}

fun env(key: String): String = System.getenv(key) ?: dopplerSecrets[key] ?: error("$key is not set")

plugins {
  alias(libs.plugins.android.application)
  alias(libs.plugins.compose.multiplatform)
  alias(libs.plugins.compose.compiler)
  alias(libs.plugins.google.services)
  alias(libs.plugins.sentryAndroid)
}

kotlin {
  target { compilerOptions { jvmTarget.set(JvmTarget.JVM_11) } }

  dependencies {
    implementation(projects.compose)

    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.splashscreen)
    implementation(libs.compose.uiToolingPreview)
    implementation(libs.ktor.client.okhttp)

    implementation(libs.kakao.user)
    implementation(libs.naver.oauth)

    implementation(platform(libs.firebase.bom))
    implementation(libs.firebase.analytics)
    implementation(libs.firebase.messaging)
  }
}

android {
  namespace = "co.typie"
  compileSdk = libs.versions.android.compileSdk.get().toInt()

  defaultConfig {
    applicationId = "co.typie"
    minSdk = libs.versions.android.minSdk.get().toInt()
    targetSdk = libs.versions.android.targetSdk.get().toInt()
    versionCode = (findProperty("versionCode") as? String)?.toInt() ?: 1
    versionName = versionProps["versionName"] as String

    manifestPlaceholders["KAKAO_NATIVE_APP_KEY"] = env("KAKAO_NATIVE_APP_KEY")
  }

  sourceSets["main"].jniLibs.directories.add("src/main/jniLibs")

  packaging { resources { excludes += "/META-INF/{AL2.0,LGPL2.1}" } }

  signingConfigs {
    getByName("debug") {
      // Keep a stable debug signature for local update-path testing.
      storeFile = debugKeystore
      storePassword = "password"
      keyAlias = "co.typie"
      keyPassword = "password"
    }

    create("release") {
      storeFile = file("keystore-release.jks")
      storePassword = System.getenv("KEYSTORE_PASSWORD")
      keyAlias = "co.typie"
      keyPassword = System.getenv("KEYSTORE_PASSWORD")
    }
  }

  buildTypes {
    getByName("debug") { signingConfig = signingConfigs.getByName("debug") }

    getByName("release") {
      isMinifyEnabled = true
      isShrinkResources = true
      proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
      signingConfig = signingConfigs.getByName("release")
    }
  }

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
  }
}

val sentryAuthToken = providers.environmentVariable("SENTRY_AUTH_TOKEN")

sentry {
  org = "typie"
  projectName = "app2"
  authToken = sentryAuthToken
  autoUploadProguardMapping = sentryAuthToken.map { true }.orElse(false)
  telemetry = false

  // The KMP SDK pins its own sentry-android version.
  autoInstallation { enabled = false }
  tracingInstrumentation { enabled = false }
}
