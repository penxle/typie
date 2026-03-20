@file:Suppress("UnstableApiUsage")

rootProject.name = "typie"
enableFeaturePreview("TYPESAFE_PROJECT_ACCESSORS")

pluginManagement {
  repositories {
    maven("https://packages.jetbrains.team/maven/p/kt/dev")
    mavenCentral()
    google {
      mavenContent {
        includeGroupAndSubgroups("androidx")
        includeGroupAndSubgroups("com.android")
        includeGroupAndSubgroups("com.google")
      }
    }
    gradlePluginPortal()
  }
}

dependencyResolutionManagement {
  repositories {
    maven("https://packages.jetbrains.team/maven/p/kt/dev")
    mavenCentral()
    google {
      mavenContent {
        includeGroupAndSubgroups("androidx")
        includeGroupAndSubgroups("com.android")
        includeGroupAndSubgroups("com.google")
      }
    }
    maven("https://devrepo.kakao.com/nexus/content/groups/public/")
  }
}

plugins {
  id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}

include(":compose")
include(":android")
