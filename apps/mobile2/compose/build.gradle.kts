@file:OptIn(ExperimentalKotlinGradlePluginApi::class, ApolloExperimental::class)
@file:Suppress("UnstableApiUsage", "ApolloEndpointNotConfigured")

import com.android.build.api.withAndroid
import com.apollographql.apollo.annotations.ApolloExperimental
import com.codingfeline.buildkonfig.compiler.FieldSpec.Type.STRING
import java.util.Properties
import org.jetbrains.kotlin.gradle.ExperimentalKotlinGradlePluginApi
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

val doppler: String =
  listOf("/opt/homebrew/bin/doppler", "/usr/local/bin/doppler").firstOrNull { file(it).exists() }
    ?: "doppler"

val dopplerSecrets: Map<String, String> by lazy {
  val output =
    providers
      .exec {
        commandLine(doppler, "secrets", "download", "-c", "dev", "--no-file", "--format", "json")
      }
      .standardOutput
      .asText
      .get()
  @Suppress("UNCHECKED_CAST")
  groovy.json.JsonSlurper().parseText(output) as Map<String, String>
}

fun env(key: String): String = System.getenv(key) ?: dopplerSecrets[key] ?: error("$key is not set")

val aboutLibrariesComposeResourceFile =
  layout.projectDirectory.file("src/commonMain/composeResources/files/aboutlibraries.json")

plugins {
  alias(libs.plugins.kotlin.multiplatform)
  alias(libs.plugins.android.multiplatformLibrary)
  alias(libs.plugins.compose.multiplatform)
  alias(libs.plugins.compose.compiler)
  alias(libs.plugins.kotlin.serialization)
  alias(libs.plugins.apollo)
  alias(libs.plugins.aboutLibraries)
  alias(libs.plugins.buildkonfig)
}

kotlin {
  applyDefaultHierarchyTemplate {
    common {
      group("jna") {
        withAndroid()
        withJvm()
      }
    }
  }

  compilerOptions {
    freeCompilerArgs.add("-Xexpect-actual-classes")
    freeCompilerArgs.add("-Xcontext-parameters")
    freeCompilerArgs.add("-opt-in=kotlin.time.ExperimentalTime")
  }

  android {
    namespace = "co.typie.compose"
    compileSdk = libs.versions.android.compileSdk.get().toInt()
    minSdk = libs.versions.android.minSdk.get().toInt()

    compilerOptions { jvmTarget.set(JvmTarget.JVM_11) }

    androidResources { enable = true }
  }

  jvm("desktop")

  listOf(iosArm64(), iosSimulatorArm64()).forEach { target ->
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
    commonMain { kotlin.srcDir(layout.buildDirectory.dir("generated/editor-bindgen/commonMain")) }

    val jnaMain by getting {
      kotlin.srcDir(rootProject.layout.projectDirectory.dir("generated/uniffi/kotlin"))
      kotlin.srcDir(layout.buildDirectory.dir("generated/editor-bindgen/jnaMain"))
    }

    androidMain {
      dependencies {
        implementation(libs.androidx.activity.compose)
        implementation(libs.androidx.credentials)
        implementation(libs.androidx.credentials.playServicesAuth)
        implementation(libs.billing)
        implementation(libs.googleid)
        implementation(libs.kakao.user)
        implementation(libs.naver.oauth)
        implementation(libs.jna.map { "$it@aar" })
      }
    }

    iosMain {
      kotlin.srcDir(layout.buildDirectory.dir("generated/editor-bindgen/iosMain"))

      dependencies { implementation(libs.ktor.client.darwin) }
    }

    val desktopMain by getting {
      resources.srcDir("src/jvmMain/resources")

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
    buildConfigField(STRING, "WS_URL", env("WS_URL"))
    buildConfigField(STRING, "AUTH_URL", env("AUTH_URL"))
    buildConfigField(STRING, "OIDC_CLIENT_ID", env("OIDC_CLIENT_ID"))
    buildConfigField(STRING, "OIDC_CLIENT_SECRET", env("OIDC_CLIENT_SECRET"))
    buildConfigField(STRING, "GOOGLE_ANDROID_CLIENT_ID", env("GOOGLE_ANDROID_CLIENT_ID"))
    buildConfigField(STRING, "GOOGLE_SERVER_CLIENT_ID", env("GOOGLE_SERVER_CLIENT_ID"))
    buildConfigField(STRING, "KAKAO_NATIVE_APP_KEY", env("KAKAO_NATIVE_APP_KEY"))
    buildConfigField(STRING, "NAVER_CLIENT_ID", env("NAVER_CLIENT_ID"))
    buildConfigField(STRING, "NAVER_CLIENT_SECRET", env("NAVER_CLIENT_SECRET"))
    buildConfigField(STRING, "USERSITE_HOST", env("USERSITE_HOST"))
  }
}

aboutLibraries { export { outputFile = aboutLibrariesComposeResourceFile.asFile } }

tasks.named("copyNonXmlValueResourcesForCommonMain") { dependsOn("exportLibraryDefinitions") }

val versionProps = Properties().apply { load(rootProject.file("version.properties").reader()) }

rootProject
  .file("ios/Configuration/Config.local.xcconfig")
  .writeText(
    """
  |MARKETING_VERSION=${versionProps["versionName"]}
  |GOOGLE_IOS_CLIENT_ID=${env("GOOGLE_IOS_CLIENT_ID")}
  |GOOGLE_DOT_REVERSED_IOS_CLIENT_ID=${env("GOOGLE_DOT_REVERSED_IOS_CLIENT_ID")}
  |GOOGLE_SERVER_CLIENT_ID=${env("GOOGLE_SERVER_CLIENT_ID")}
  |KAKAO_NATIVE_APP_KEY=${env("KAKAO_NATIVE_APP_KEY")}
  |NAVER_CLIENT_ID=${env("NAVER_CLIENT_ID")}
  |NAVER_CLIENT_SECRET=${env("NAVER_CLIENT_SECRET")}
  """
      .trimMargin()
  )

apollo {
  service("typie") {
    packageName = "co.typie.graphql"

    srcDir("src/commonMain/kotlin")
    schemaFiles.from(
      "src/commonMain/graphql/schema.graphqls",
      "src/commonMain/graphql/apollo.graphqls",
    )

    mapScalar("DateTime", "kotlin.time.Instant", "co.typie.graphql.adapter.InstantAdapter")
    mapScalar(
      "JSON",
      "kotlinx.serialization.json.JsonElement",
      "co.typie.graphql.adapter.JsonElementAdapter",
    )

    addTypename = "always"
    generateDataBuilders = true
    generateInputBuilders = true
    generateFragmentImplementations = true

    plugin(
      "com.apollographql.cache:normalized-cache-apollo-compiler-plugin:${libs.versions.apollo.normalized.cache.get()}"
    )
    pluginArgument("com.apollographql.cache.packageName", packageName.get())

    dataBuildersOutputDirConnection { connectToKotlinSourceSet("commonMain") }
  }
}

compose.resources { packageOfResClass = "co.typie.generated.resources" }

compose.desktop { application { mainClass = "co.typie.MainKt" } }

dependencies { androidRuntimeClasspath(libs.compose.uiTooling) }
