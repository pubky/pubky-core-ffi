plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("com.vanniktech.maven.publish")
}

fun cargoPackageVersion(): String {
    val cargoManifest = file("../../Cargo.toml").readText()
    return Regex("""(?ms)^\[package]\s*.*?^version\s*=\s*"([^"]+)"""")
        .find(cargoManifest)
        ?.groupValues
        ?.get(1)
        ?: error("Unable to read the package version from Cargo.toml")
}

val publicationVersion = providers.gradleProperty("VERSION_NAME")
    .orElse(cargoPackageVersion())
    .get()

android {
    namespace = "org.rustls.platformverifier"
    compileSdk = 36

    defaultConfig {
        minSdk = 22
        buildConfigField("boolean", "TEST", "false")
        consumerProguardFiles("consumer-rules.pro")
    }

    buildFeatures {
        buildConfig = true
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    kotlinOptions {
        jvmTarget = "11"
    }

    sourceSets.named("main") {
        java.srcDirs("../../bindings/android", "src/main/kotlin")
        jniLibs.srcDir("../../bindings/android/jniLibs")
    }
}

dependencies {
    api("net.java.dev.jna:jna:5.18.1@aar")
}

mavenPublishing {
    coordinates("org.pubky", "pubky-core-android", publicationVersion)

    pom {
        name.set("Pubky Core Android")
        description.set("Android bindings for Pubky Core")
        inceptionYear.set("2024")
        url.set("https://github.com/pubky/pubky-core-ffi")

        licenses {
            license {
                name.set("MIT License")
                url.set("https://opensource.org/licenses/MIT")
                distribution.set("repo")
            }
        }

        developers {
            developer {
                id.set("pubky")
                name.set("Pubky")
                url.set("https://github.com/pubky")
            }
        }

        scm {
            url.set("https://github.com/pubky/pubky-core-ffi")
            connection.set("scm:git:https://github.com/pubky/pubky-core-ffi.git")
            developerConnection.set("scm:git:ssh://git@github.com/pubky/pubky-core-ffi.git")
        }
    }
}

publishing {
    repositories {
        maven {
            name = "build"
            url = uri(layout.buildDirectory.dir("repo"))
        }
        maven {
            name = "GitHubPackages"
            val repository = System.getenv("GITHUB_REPOSITORY")
                ?: providers.gradleProperty("gpr.repo").orNull
                ?: "pubky/pubky-core-ffi"
            url = uri("https://maven.pkg.github.com/$repository")
            credentials {
                username = System.getenv("GITHUB_ACTOR")
                    ?: providers.gradleProperty("gpr.user").orNull
                password = System.getenv("GITHUB_TOKEN")
                    ?: providers.gradleProperty("gpr.key").orNull
            }
        }
    }
}
