import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
  alias(libs.plugins.android.application)
  alias(libs.plugins.compose.multiplatform)
  alias(libs.plugins.compose.compiler)
}

kotlin {
  target {
    compilerOptions {
      jvmTarget.set(JvmTarget.JVM_11)
    }
  }

  dependencies {
    implementation(projects.compose)

    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.splashscreen)
    implementation(libs.compose.uiToolingPreview)
    implementation(libs.koin.android)
    implementation(libs.ktor.client.okhttp)

    implementation(libs.kakao.user)
    implementation(libs.naver.oauth)
  }
}

android {
  namespace = "co.typie"
  compileSdk = libs.versions.android.compileSdk.get().toInt()

  defaultConfig {
    applicationId = "co.typie"
    minSdk = libs.versions.android.minSdk.get().toInt()
    targetSdk = libs.versions.android.targetSdk.get().toInt()
    versionCode = 1
    versionName = "1.0"
  }

  sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")

  packaging {
    resources {
      excludes += "/META-INF/{AL2.0,LGPL2.1}"
    }
  }

  buildTypes {
    getByName("release") {
      isMinifyEnabled = false
    }
  }

  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
  }
}

