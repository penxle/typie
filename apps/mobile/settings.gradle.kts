@file:Suppress("UnstableApiUsage")

rootProject.name = "typie"

enableFeaturePreview("TYPESAFE_PROJECT_ACCESSORS")

val githubPackagesUser =
  providers.gradleProperty("gpr.user").orElse(providers.environmentVariable("GITHUB_ACTOR"))
val githubPackagesToken =
  providers.gradleProperty("gpr.key").orElse(providers.environmentVariable("GITHUB_TOKEN"))
val composePatchesRepository = providers.gradleProperty("composePatchesRepository").orNull

pluginManagement {
  repositories {
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
    exclusiveContent {
      forRepository {
        if (composePatchesRepository != null) {
          maven(composePatchesRepository) { name = "ComposePatchesLocal" }
        } else {
          maven("https://maven.pkg.github.com/penxle/compose-patches") {
            name = "GitHubPackages"
            // Local builds read these from ~/.gradle/gradle.properties; CI can provide the
            // environment fallbacks above.
            credentials {
              username = githubPackagesUser.orNull
              password = githubPackagesToken.orNull
            }
          }
        }
      }
      filter {
        includeVersion("org.jetbrains.compose.ui", "ui-iosarm64", "1.12.0")
        includeVersion("org.jetbrains.compose.ui", "ui-iossimulatorarm64", "1.12.0")
      }
    }
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

plugins { id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0" }

include(":compose")

include(":android")
