//! Leftovers Windows keeps outside the obvious folders: setup, update and servicing logs, kernel
//! dumps, the Windows Update download cache, the previous Windows installation and the payloads
//! installers keep for repair. Logs and dumps are cheap to lose; the rest is opt-in because it is
//! big, slow to download again or removes the way back to the previous version.
//!
//! `C:\Windows\Installer`, `ProgramData\Package Cache` and `WinSxS` itself stay out of scope: Windows
//! needs them to repair, service and uninstall programs, so they are cleaned by the OS tools only.

use crate::junk_defs::JunkDef;

#[cfg(windows)]
use crate::junk_defs::{caches, def, matching, opt_in, whole, Base, INSTALLERS, SYS};
#[cfg(windows)]
use std::time::Duration;

pub fn definitions() -> Vec<JunkDef> {
    windows()
}

#[cfg(not(windows))]
fn windows() -> Vec<JunkDef> {
    Vec::new()
}

#[cfg(windows)]
fn windows() -> Vec<JunkDef> {
    const DAY: Duration = Duration::from_secs(24 * 60 * 60);
    const WEEK: Duration = Duration::from_secs(7 * 24 * 60 * 60);

    // Windows Setup rewrites Panther while an upgrade runs, so only week-old files go.
    let setup_logs = JunkDef {
        min_age: WEEK,
        ..def(
            "win-setup-logs",
            "Logs de instalação e atualização do Windows",
            SYS,
            Base::Windows,
            vec![
                whole("Panther"),
                whole("Logs/CBS"),
                whole("Logs/DISM"),
                whole("Logs/WindowsUpdate"),
                whole("Logs/MoSetup"),
                whole("Logs/waasmedic"),
                whole("Logs/SIH"),
                whole("Downloaded Program Files"),
                whole("Offline Web Pages"),
            ],
        )
    };

    let shared_logs = JunkDef {
        min_age: DAY,
        ..def(
            "app-update-logs",
            "Logs de atualizações e instaladores",
            SYS,
            Base::ProgramData,
            vec![
                whole("USOShared/Logs"),
                whole("chocolatey/logs"),
                whole("Microsoft/EdgeUpdate/Logs"),
            ],
        )
    };

    let kernel_dumps = def(
        "kernel-reports",
        "Despejos de falha do kernel",
        SYS,
        Base::Windows,
        vec![
            matching("Minidump", &["*.dmp"], true),
            matching("LiveKernelReports", &["*.dmp"], true),
        ],
    );

    let prefetch = def(
        "prefetch",
        "Pré-carregamento de programas (Prefetch)",
        SYS,
        Base::Windows,
        vec![matching("Prefetch", &["*.pf"], false)],
    );

    let shared_wer = def(
        "wer-shared",
        "Relatórios de erro de todos os usuários",
        SYS,
        Base::ProgramData,
        vec![
            whole("Microsoft/Windows/WER/ReportArchive"),
            whole("Microsoft/Windows/WER/ReportQueue"),
            whole("Microsoft/Windows/WER/Temp"),
        ],
    );

    // Opt-in: what an installer left behind, a big update download or the previous Windows.
    let previous_windows = opt_in(
        caches(
            "windows-old",
            "Instalação anterior do Windows (Windows.old)",
            INSTALLERS,
            Base::SystemDrive,
            &["Windows.old"],
        ),
        "É a cópia da versão anterior do Windows. Depois de apagá-la não há como voltar atrás pela \
         restauração do sistema; o próprio Windows a remove sozinho 10 dias após a atualização.",
    );

    let update_downloads = JunkDef {
        min_age: DAY,
        ..opt_in(
            caches(
                "windows-update-cache",
                "Downloads do Windows Update",
                INSTALLERS,
                Base::Windows,
                &["SoftwareDistribution/Download"],
            ),
            "Atualizações já baixadas serão baixadas de novo. Não limpe com uma atualização \
             pendente e feche o Windows Update antes; sem permissão de administrador a limpeza falha.",
        )
    };

    let system_temp = JunkDef {
        min_age: DAY,
        ..opt_in(
            caches(
                "windows-system-temp",
                "Temporários do sistema (SystemTemp)",
                SYS,
                Base::Windows,
                &["SystemTemp"],
            ),
            "Arquivos temporários usados por serviços do Windows (Windows 11 24H2 e mais novos). \
             Itens com menos de um dia ficam; sem permissão de administrador a limpeza falha.",
        )
    };

    let delivery = opt_in(
        caches(
            "delivery-optimization",
            "Cache da Otimização de Entrega (atualizações)",
            INSTALLERS,
            Base::ProgramData,
            &["Microsoft/Network/Downloader"],
        ),
        "São blocos de atualizações baixados de outros computadores e da internet. O Windows baixa \
         de novo quando precisar; não limpe com uma atualização em andamento.",
    );

    let installer_payloads = opt_in(
        caches(
            "nvidia-downloader",
            "Instaladores baixados pelo NVIDIA (Downloader)",
            INSTALLERS,
            Base::ProgramData,
            &["NVIDIA Corporation/Downloader"],
        ),
        "São os pacotes de driver que o NVIDIA baixa antes de instalar. A próxima atualização baixa \
         de novo; sem eles o instalador pode pedir o pacote original.",
    );

    let downloaded_installations = opt_in(
        caches(
            "downloaded-installations",
            "Instaladores MSI guardados por aplicativos",
            INSTALLERS,
            Base::Local,
            &["Downloaded Installations"],
        ),
        "Cópias de instaladores que alguns programas guardam ao serem instalados. Reparar ou \
         desinstalar esses programas pode pedir o instalador original.",
    );

    let esd = JunkDef {
        min_age: WEEK,
        ..opt_in(
            caches(
                "esd-downloads",
                "Downloads de atualização do Windows (ESD)",
                INSTALLERS,
                Base::SystemDrive,
                &["ESD"],
            ),
            "São os arquivos de atualização baixados para instalar o Windows. A próxima atualização \
             baixa tudo de novo.",
        )
    };

    let adobe = opt_in(
        caches(
            "adobe-installers",
            "Instaladores e atualizações da Adobe",
            INSTALLERS,
            Base::ProgramData,
            &["Adobe/ARM"],
        ),
        "Pacotes baixados pelo Adobe Reader/Acrobat para se atualizar. A próxima atualização baixa \
         de novo.",
    );

    let drivers = opt_in(
        caches(
            "driver-installers",
            "Pacotes de driver extraídos (NVIDIA, Intel)",
            INSTALLERS,
            Base::SystemDrive,
            &["NVIDIA", "Intel"],
        ),
        "Arquivos extraídos para instalar ou recuperar drivers de vídeo e chipset. Limpe só depois \
         de concluir a instalação e reiniciar; será preciso baixar o pacote novamente para reparar.",
    );

    vec![
        setup_logs,
        shared_logs,
        kernel_dumps,
        prefetch,
        shared_wer,
        previous_windows,
        update_downloads,
        system_temp,
        delivery,
        installer_payloads,
        downloaded_installations,
        esd,
        adobe,
        drivers,
    ]
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::junk_defs::{Scope, SYS};

    #[test]
    fn every_installer_leftover_is_opt_in_with_a_warning() {
        let defs = windows();
        let installers: Vec<_> = defs.iter().filter(|d| d.group == INSTALLERS).collect();
        assert_eq!(
            installers.len(),
            8,
            "the installer group lost entries: {:?}",
            installers.iter().map(|d| d.id).collect::<Vec<_>>()
        );
        assert!(
            installers
                .iter()
                .all(|d| d.caution.is_some_and(|c| c.len() > 40)),
            "every installer leftover must explain itself before the user ticks it"
        );
        // Reading from %SystemRoot% and %ProgramData% is what these definitions are for.
        assert!(installers
            .iter()
            .any(|d| matches!(d.base, Base::Windows | Base::ProgramData)));
    }

    #[test]
    fn logs_and_dumps_stay_off_the_opt_in_list() {
        let defs = windows();
        for id in [
            "win-setup-logs",
            "app-update-logs",
            "kernel-reports",
            "prefetch",
            "wer-shared",
        ] {
            let d = defs.iter().find(|d| d.id == id).expect(id);
            assert_eq!(d.group, SYS, "{id}");
            assert!(d.caution.is_none(), "{id} is cheap to lose");
        }
    }

    #[test]
    fn dump_and_prefetch_rules_delete_files_only() {
        let defs = windows();
        let d = defs.iter().find(|d| d.id == "kernel-reports").unwrap();
        assert!(d.rules.iter().all(|r| r.scope == Scope::Files && r.recurse));
        let p = defs.iter().find(|d| d.id == "prefetch").unwrap();
        assert_eq!(p.rules[0].patterns, ["*.pf"]);
    }

    #[test]
    fn system_locations_age_before_they_are_touched() {
        let defs = windows();
        let by_id = |id: &str| defs.iter().find(|d| d.id == id).unwrap();
        for id in [
            "windows-update-cache",
            "windows-system-temp",
            "win-setup-logs",
            "esd-downloads",
        ] {
            assert!(
                by_id(id).min_age >= std::time::Duration::from_secs(24 * 60 * 60),
                "{id} would touch files a running installer may still own"
            );
        }
    }
}
