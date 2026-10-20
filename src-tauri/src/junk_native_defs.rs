//! Toolchain caches for native, mobile, JVM, infra and game work. Every entry is re-downloaded or
//! rebuilt on demand. Conan's `p` folder and Xcode Archives are left out: they hold locally
//! created packages and release builds.

use crate::junk_defs::{caches, Base, JunkDef, DEV};

pub fn definitions() -> Vec<JunkDef> {
    let mut v = vec![
        caches(
            "gradle-daemon",
            "Gradle (daemon e temporários)",
            DEV,
            Base::Home,
            &[".gradle/daemon", ".gradle/.tmp"],
        ),
        caches(
            "android-cache",
            "Android (cache e build cache)",
            DEV,
            Base::Home,
            &[".android/cache", ".android/build-cache"],
        ),
        caches(
            "kotlin-native",
            "Kotlin/Native (dependências e cache)",
            DEV,
            Base::Home,
            &[".konan/cache", ".konan/dependencies"],
        ),
        caches(
            "sdkman",
            "SDKMAN (downloads e temporários)",
            DEV,
            Base::Home,
            &[".sdkman/archives", ".sdkman/tmp"],
        ),
        caches(
            "cargo-index",
            "Cargo (índice do registry)",
            DEV,
            Base::Home,
            &[".cargo/registry/index"],
        ),
        caches(
            "ruby-hex-cabal",
            "Ruby, Elixir e Haskell (pacotes baixados)",
            DEV,
            Base::Home,
            &[".bundle/cache", ".hex/packages", ".cabal/packages"],
        ),
        caches(
            "infra-caches",
            "Terraform, Pulumi, Kubernetes e Minikube (cache)",
            DEV,
            Base::Home,
            &[
                ".terraform.d/plugin-cache",
                ".pulumi/plugins",
                ".kube/cache",
                ".minikube/cache",
            ],
        ),
        caches(
            "version-managers",
            "nvm e pyenv (downloads)",
            DEV,
            Base::Home,
            &[".nvm/.cache", ".pyenv/cache"],
        ),
        caches(
            "ccache-legacy",
            "ccache (formato antigo)",
            DEV,
            Base::Home,
            &[".ccache"],
        ),
    ];
    v.extend(platform());
    v
}

#[cfg(target_os = "macos")]
fn platform() -> Vec<JunkDef> {
    vec![
        caches(
            "xcode-extra",
            "Xcode (símbolos de dispositivos, previews e simuladores)",
            DEV,
            Base::Home,
            &[
                "Library/Developer/Xcode/iOS DeviceSupport",
                "Library/Developer/Xcode/watchOS DeviceSupport",
                "Library/Developer/Xcode/UserData/Previews",
                "Library/Developer/CoreSimulator/Caches",
            ],
        ),
        caches(
            "cocoapods",
            "CocoaPods (repositório de specs)",
            DEV,
            Base::Home,
            &[".cocoapods/repos"],
        ),
        caches(
            "unity-cache",
            "Unity (cache de assets)",
            DEV,
            Base::Home,
            &["Library/Unity/cache"],
        ),
        caches(
            "native-home-cache",
            "vcpkg, ccache, Bazel, Deno e Helm (cache)",
            DEV,
            Base::Home,
            &[
                ".cache/vcpkg/archives",
                ".cache/ccache",
                ".cache/bazel",
                ".cache/bazelisk",
                ".cache/deno",
                ".cache/helm",
            ],
        ),
    ]
}

#[cfg(windows)]
fn platform() -> Vec<JunkDef> {
    vec![
        caches(
            "native-local",
            "vcpkg, ccache e Deno (cache)",
            DEV,
            Base::Local,
            &["vcpkg/archives", "ccache", "deno"],
        ),
        caches(
            "native-home-cache",
            "Bazel e Helm (cache)",
            DEV,
            Base::Home,
            &[".cache/bazel", ".cache/bazelisk", ".cache/helm"],
        ),
        caches(
            "scoop",
            "Scoop (instaladores baixados)",
            DEV,
            Base::Home,
            &["scoop/cache"],
        ),
        caches(
            "visualstudio",
            "Visual Studio (cache de componentes) e TypeScript",
            DEV,
            Base::Local,
            &[
                "Microsoft/VisualStudio/*/ComponentModelCache",
                "Microsoft/TypeScript",
            ],
        ),
        caches(
            "unity-cache",
            "Unity (cache de assets)",
            DEV,
            Base::Local,
            &["Unity/cache"],
        ),
        caches(
            "gpu-vendor",
            "Cache de shaders Vulkan/OptiX (AMD, NVIDIA)",
            DEV,
            Base::Local,
            &["AMD/VkCache", "NVIDIA/OptixCache"],
        ),
    ]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform() -> Vec<JunkDef> {
    Vec::new()
}
