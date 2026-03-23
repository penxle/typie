@file:OptIn(ExperimentalKotlinGradlePluginApi::class)

import com.codingfeline.buildkonfig.compiler.FieldSpec.Type.STRING
import org.jetbrains.kotlin.gradle.ExperimentalKotlinGradlePluginApi
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

val doppler: String = listOf("/opt/homebrew/bin/doppler", "/usr/local/bin/doppler")
  .firstOrNull { file(it).exists() } ?: "doppler"

val dopplerSecrets: Map<String, String> by lazy {
  val output = providers.exec {
    commandLine(doppler, "secrets", "download", "--no-file", "--format", "json")
  }.standardOutput.asText.get()
  @Suppress("UNCHECKED_CAST")
  groovy.json.JsonSlurper().parseText(output) as Map<String, String>
}

fun env(key: String): String =
  System.getenv(key) ?: dopplerSecrets[key] ?: error("$key is not set")

plugins {
  alias(libs.plugins.kotlin.multiplatform)
  alias(libs.plugins.android.multiplatformLibrary)
  alias(libs.plugins.compose.multiplatform)
  alias(libs.plugins.compose.compiler)
  alias(libs.plugins.koin.compiler)
  alias(libs.plugins.kotlin.serialization)
  alias(libs.plugins.apollo)
  alias(libs.plugins.buildkonfig)
}

kotlin {
  compilerOptions {
    freeCompilerArgs.add("-Xexpect-actual-classes")
    freeCompilerArgs.add("-opt-in=kotlin.time.ExperimentalTime")
  }

  android {
    namespace = "co.typie.compose"
    compileSdk = libs.versions.android.compileSdk.get().toInt()
    minSdk = libs.versions.android.minSdk.get().toInt()

    compilerOptions {
      jvmTarget.set(JvmTarget.JVM_11)
    }

    androidResources {
      enable = true
    }
  }

  jvm()

  listOf(
    iosArm64(),
    iosSimulatorArm64()
  ).forEach { target ->
    target.binaries.framework {
      baseName = "Compose"
      isStatic = true
      binaryOption("bundleId", "co.typie.compose")
    }
  }

  swiftPMDependencies {
    group = "co.typie"
    iosMinimumDeploymentTarget.set("15.6")

    localSwiftPackage(
      directory = rootProject.layout.projectDirectory.dir("ios/Bridge"),
      products = listOf("Bridge"),
    )
  }

  sourceSets {
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

    iosMain {
      dependencies {
        implementation(libs.ktor.client.darwin)
      }
    }

    jvmMain {
      dependencies {
        implementation(compose.desktop.currentOs)
        implementation(libs.jna)
        implementation(libs.kotlinx.coroutines.swing)
        implementation(libs.ktor.client.cio)
      }
    }
  }

  dependencies {
    implementation(libs.androidx.lifecycle.runtimeCompose)
    implementation(libs.androidx.lifecycle.viewmodelCompose)
    implementation(libs.coil.resvg)
    implementation(libs.coil3.compose)
    implementation(libs.coil3.network.ktor3)
    implementation(libs.compose.components.resources)
    implementation(libs.compose.foundation)
    implementation(libs.compose.runtime)
    implementation(libs.compose.ui)
    implementation(libs.compose.uiToolingPreview)
    implementation(libs.kermit)
    implementation(libs.koin.annotations)
    implementation(libs.koin.compose)
    implementation(libs.koin.compose.viewmodel)
    implementation(libs.koin.core)
    implementation(libs.kotlinx.coroutines.core)
    implementation(libs.kotlinx.datetime)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.ktor.client.core)
    implementation(libs.apollo.runtime)
    implementation(libs.apollo.engine.ktor)
    implementation(libs.apollo.normalized.cache)
    implementation(libs.ksafe)
    implementation(libs.haze)

    testImplementation(libs.kotlin.test)
    testImplementation(libs.kotlinx.coroutines.test)
  }
}

buildkonfig {
  packageName = "co.typie"
  exposeObjectWithName = "Konfig"

  defaultConfigs {
    buildConfigField(STRING, "API_URL", env("API_URL"))
    buildConfigField(STRING, "AUTH_URL", env("AUTH_URL"))
    buildConfigField(STRING, "OIDC_CLIENT_ID", env("OIDC_CLIENT_ID"))
    buildConfigField(STRING, "OIDC_CLIENT_SECRET", env("OIDC_CLIENT_SECRET"))
    buildConfigField(STRING, "GOOGLE_ANDROID_CLIENT_ID", env("GOOGLE_ANDROID_CLIENT_ID"))
    buildConfigField(STRING, "GOOGLE_SERVER_CLIENT_ID", env("GOOGLE_SERVER_CLIENT_ID"))
    buildConfigField(STRING, "KAKAO_NATIVE_APP_KEY", env("KAKAO_NATIVE_APP_KEY"))
    buildConfigField(STRING, "NAVER_CLIENT_ID", env("NAVER_CLIENT_ID"))
    buildConfigField(STRING, "NAVER_CLIENT_SECRET", env("NAVER_CLIENT_SECRET"))
  }
}

rootProject.file("ios/Configuration/Config.local.xcconfig").writeText(
  """
  |GOOGLE_IOS_CLIENT_ID=${env("GOOGLE_IOS_CLIENT_ID")}
  |GOOGLE_DOT_REVERSED_IOS_CLIENT_ID=${env("GOOGLE_DOT_REVERSED_IOS_CLIENT_ID")}
  |GOOGLE_SERVER_CLIENT_ID=${env("GOOGLE_SERVER_CLIENT_ID")}
  |KAKAO_NATIVE_APP_KEY=${env("KAKAO_NATIVE_APP_KEY")}
  |NAVER_CLIENT_ID=${env("NAVER_CLIENT_ID")}
  |NAVER_CLIENT_SECRET=${env("NAVER_CLIENT_SECRET")}
  """.trimMargin()
)

apollo {
  service("typie") {
    packageName = "co.typie.graphql"

    srcDir("src/commonMain/kotlin")
    schemaFiles.from(
      "src/commonMain/graphql/schema.graphqls",
      "src/commonMain/graphql/apollo.graphqls"
    )

    mapScalar("DateTime", "kotlin.time.Instant", "co.typie.graphql.adapter.InstantAdapter")
    mapScalar(
      "JSON",
      "kotlinx.serialization.json.JsonElement",
      "co.typie.graphql.adapter.JsonElementAdapter"
    )

    addTypename = "always"
    generateDataBuilders = true
    generateInputBuilders = true
    generateFragmentImplementations = true
  }
}

compose.resources {
  packageOfResClass = "co.typie.generated.resources"
}

compose.desktop {
  application {
    mainClass = "co.typie.MainKt"
  }
}

dependencies {
  androidRuntimeClasspath(libs.compose.uiTooling)
}
