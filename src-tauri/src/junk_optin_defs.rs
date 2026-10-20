//! Opt-in locations: regenerable, but big, slow or inconvenient to get back, or holding something
//! the user chose to keep. They are never ticked by default and each carries a warning.
//! Still out of bounds: chat transcripts, cloud credential caches and browser/profile data.

use crate::junk_defs::{caches, def, matching, opt_in, Base, JunkDef, APPS, DEV};

const MODELS: &str = "Modelos que você baixou. Voltam a ser baixados na próxima vez que usar, o que pode levar horas.";

pub fn definitions() -> Vec<JunkDef> {
    let mut v = vec![
        opt_in(
            caches("ollama-models", "Modelos do Ollama", DEV, Base::Home, &[".ollama/models"]),
            MODELS,
        ),
        opt_in(
            caches("lmstudio-models", "Modelos do LM Studio", DEV, Base::Home, &[".lmstudio/models"]),
            MODELS,
        ),
        opt_in(
            def(
                "gpt4all-models",
                "Modelos do GPT4All",
                DEV,
                Base::Local,
                vec![matching("nomic.ai/GPT4All", &["*.gguf", "*.bin"], false)],
            ),
            "Só os arquivos de modelo (.gguf/.bin); conversas e documentos ficam. Precisam ser baixados de novo.",
        ),
        opt_in(
            caches("android-avd", "Emuladores Android (AVDs)", DEV, Base::Home, &[".android/avd"]),
            "Apaga os emuladores e os apps e dados instalados neles. É preciso recriá-los no Android Studio.",
        ),
        opt_in(
            caches("vagrant-boxes", "Vagrant (boxes)", DEV, Base::Home, &[".vagrant.d/boxes"]),
            "As boxes são baixadas de novo ao subir a VM; máquinas já criadas podem deixar de iniciar.",
        ),
        opt_in(
            caches("conan-packages", "Conan (pacotes e receitas locais)", DEV, Base::Home, &[".conan2/p", ".conan/data"]),
            "Inclui pacotes que você criou localmente com `conan create`; esses não podem ser baixados de novo.",
        ),
    ];
    v.extend(platform());
    v
}

#[cfg(windows)]
fn platform() -> Vec<JunkDef> {
    vec![
        opt_in(
            caches("android-images", "Imagens de sistema do Android SDK", DEV, Base::Local, &["Android/Sdk/system-images"]),
            "Cada imagem tem GBs e precisa ser baixada de novo pelo SDK Manager para criar emuladores.",
        ),
        opt_in(
            def(
                "docker-disk",
                "Disco do Docker Desktop (WSL)",
                DEV,
                Base::Local,
                vec![matching("Docker/wsl", &["docker_data.vhdx", "ext4.vhdx"], true)],
            ),
            "APAGA todas as imagens, containers e VOLUMES (bancos de dados incluídos) do Docker. Feche o Docker antes; não há como recuperar os volumes.",
        ),
        opt_in(
            caches("office-upload", "Cache de arquivos do Office", APPS, Base::Local, &["Microsoft/Office/16.0/OfficeFileCache"]),
            "Pode conter alterações ainda não sincronizadas de documentos abertos no Office. Confira o Centro de Upload antes.",
        ),
        opt_in(
            caches("lmstudio-legacy", "Modelos do LM Studio (pasta antiga)", DEV, Base::Home, &[".cache/lm-studio/models"]),
            MODELS,
        ),
        opt_in(
            caches("iphone-backups", "Backups do iPhone/iPad", APPS, Base::Roaming, &["Apple Computer/MobileSync/Backup"]),
            "São cópias do seu aparelho e podem ser a única. Só apague se tiver outro backup (iCloud, por exemplo).",
        ),
    ]
}

#[cfg(target_os = "macos")]
fn platform() -> Vec<JunkDef> {
    vec![
        opt_in(
            caches("android-images", "Imagens de sistema do Android SDK", DEV, Base::Home, &["Library/Android/sdk/system-images"]),
            "Cada imagem tem GBs e precisa ser baixada de novo pelo SDK Manager para criar emuladores.",
        ),
        opt_in(
            caches("xcode-archives", "Xcode Archives", DEV, Base::Home, &["Library/Developer/Xcode/Archives"]),
            "São os builds de release com símbolos (dSYMs). Sem eles, relatórios de falha de versões publicadas ficam ilegíveis.",
        ),
        opt_in(
            caches("simulators", "Simuladores do iOS (dispositivos)", DEV, Base::Home, &["Library/Developer/CoreSimulator/Devices"]),
            "Apaga os simuladores com os apps e dados instalados neles; o Xcode recria os padrão.",
        ),
        opt_in(
            def(
                "docker-disk",
                "Disco do Docker Desktop",
                DEV,
                Base::Home,
                vec![matching(
                    "Library/Containers/com.docker.docker/Data/vms",
                    &["Docker.raw", "Docker.qcow2"],
                    true,
                )],
            ),
            "APAGA todas as imagens, containers e VOLUMES (bancos de dados incluídos) do Docker. Feche o Docker antes; não há como recuperar os volumes.",
        ),
        opt_in(
            caches("iphone-backups", "Backups do iPhone/iPad", APPS, Base::Home, &["Library/Application Support/MobileSync/Backup"]),
            "São cópias do seu aparelho e podem ser a única. Só apague se tiver outro backup (iCloud, por exemplo).",
        ),
        opt_in(
            caches("steam-partial", "Steam (downloads em andamento)", APPS, Base::Local, &["Steam/steamapps/downloading", "Steam/steamapps/temp"]),
            "Cancela atualizações e instalações pendentes; o Steam recomeça o download do zero.",
        ),
        opt_in(
            caches("lmstudio-legacy", "Modelos do LM Studio (pasta antiga)", DEV, Base::Home, &[".cache/lm-studio/models"]),
            MODELS,
        ),
    ]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform() -> Vec<JunkDef> {
    vec![
        opt_in(
            caches("android-images", "Imagens de sistema do Android SDK", DEV, Base::Home, &["Android/Sdk/system-images"]),
            "Cada imagem tem GBs e precisa ser baixada de novo pelo SDK Manager para criar emuladores.",
        ),
        opt_in(
            caches("steam-partial", "Steam (downloads em andamento)", APPS, Base::Local, &["Steam/steamapps/downloading", "Steam/steamapps/temp"]),
            "Cancela atualizações e instalações pendentes; o Steam recomeça o download do zero.",
        ),
    ]
}
