use std::{
    fmt::Write as _,
    fs::{self, OpenOptions},
    io::Write as _,
    path::Path,
};

use archrescue_deployment::{self, Intent, Media, Plan, Qualification};

use super::*;

pub const DEPLOY: usize = 6;
pub const RESTORE: usize = 7;

const MEDIA: usize = 230;
const INSPECT: usize = 231;
const IMAGE_INDEX: usize = 232;
const DISK: usize = 233;
const MODE: usize = 234;
const PRESERVATION: usize = 235;
const PLAN: usize = 236;
const SAVE: usize = 237;
const HANDOFF: usize = 238;
const INSTALL: usize = 239;
const MEDIA_LABEL: usize = 246;
const BACKUP_LABEL: usize = 247;

#[derive(Default)]
pub struct Deployment {
    media: Option<Media>,
    disk: Option<String>,
    source: Option<Location>,
    plan: Option<Plan>,
    lab_status: Option<String>,
}

pub enum Update {
    Progress(u64, u64),
    Media(Box<Media>),
    Plan(Box<Plan>),
    #[cfg(all(feature = "lab-deployment", target_arch = "x86_64"))]
    LabProgress(archrescue_deployment::lab::windows::Step),
    #[cfg(all(feature = "lab-deployment", target_arch = "x86_64"))]
    LabFinished(Box<Result<archrescue_deployment::lab::windows::Evidence>>),
}

pub unsafe fn controls(hwnd: HWND, font: HFONT) -> Result<()> {
    for (id, label, caption) in [
        (MEDIA, MEDIA_LABEL, "Mídia"),
        (PRESERVATION, BACKUP_LABEL, "Backup para revisão"),
    ] {
        add(hwnd, label, "STATIC", caption, 0, font)?;
        add(hwnd, id, "BUTTON", "", WS_TABSTOP, font)?;
    }

    for id in [IMAGE_INDEX, MODE] {
        add(hwnd, id, select::class(), "", WS_TABSTOP, font)?;
    }
    add(hwnd, DISK, "BUTTON", "Escolher disco", WS_TABSTOP, font)?;
    for mode in [
        "Somente inspeção",
        "Reparo",
        "Instalar preservando dados",
        "Instalação limpa",
    ] {
        add_option(hwnd, MODE, mode);
    }
    SendMessageW(control(hwnd, MODE), CB_SETCURSEL, 0, 0);

    for (id, label) in [
        (INSPECT, "Trocar imagem"),
        (PLAN, "&Gerar prévia"),
        (SAVE, "Exportar &plano"),
        (HANDOFF, "Verificar e exportar referência"),
        (INSTALL, "Instalar Windows"),
    ] {
        add(hwnd, id, "BUTTON", label, WS_TABSTOP, font)?;
    }

    Ok(())
}

unsafe fn add_option(hwnd: HWND, id: usize, text: &str) {
    SendMessageW(
        control(hwnd, id),
        CB_ADDSTRING,
        0,
        wide(text).as_ptr() as isize,
    );
}

unsafe fn selection(hwnd: HWND, id: usize) -> usize {
    SendMessageW(control(hwnd, id), CB_GETCURSEL, 0, 0) as usize
}

pub unsafe fn reference(hwnd: HWND, app: &App) -> Option<(Media, archrescue_deployment::Image)> {
    let media = app.deployment.media.as_ref()?;
    let image = media.images.get(selection(hwnd, IMAGE_INDEX))?;
    Some((media.clone(), image.clone()))
}

pub fn form_label(id: usize) -> bool {
    matches!(id, MEDIA_LABEL | BACKUP_LABEL)
}

pub fn form_field(id: usize) -> bool {
    matches!(id, MEDIA | PRESERVATION)
}

pub fn media_loaded(app: &App) -> bool {
    app.deployment.media.is_some()
}

pub fn qualification(app: &App) -> Option<(&'static str, &str, COLORREF)> {
    use super::theme::{ACCENT, CRITICAL, WARNING};

    let plan = app.deployment.plan.as_ref()?;
    let (label, color) = match plan.result {
        Qualification::Supported => ("Compatível para planejamento", ACCENT),
        Qualification::RequiresIntervention => ("Requer intervenção", WARNING),
        Qualification::Unsupported => ("Configuração sem suporte", WARNING),
        Qualification::Blocked => ("Plano bloqueado", CRITICAL),
    };
    let detail = plan
        .findings
        .iter()
        .find(|finding| finding.result == plan.result)
        .or_else(|| plan.findings.first())
        .map_or(plan.next_action.as_str(), |finding| finding.reason.as_str());
    Some((label, detail, color))
}

pub fn empty_choice(id: usize) -> &'static str {
    match id {
        IMAGE_INDEX => "Edição do Windows",
        DISK => "Disco GPT",
        MODE => "Modo",
        _ => "Selecione uma opção",
    }
}

pub fn primary(id: usize) -> bool {
    matches!(id, INSPECT | PLAN | SAVE | HANDOFF | INSTALL)
}

pub unsafe fn layout(hwnd: HWND, area: &super::layout::Layout, app: &App) {
    let x = area.left();
    let width = area.body_width();
    let (left, right, right_width) = area.columns(60);

    let base = area.body_top(DASHBOARD);
    if app.section == RESTORE {
        place(hwnd, BACKUP_LABEL, x + 13, base + 40, left - 26, 20);
        place(hwnd, PRESERVATION, x + 19, base + 63, left - 38, 23);
        place(hwnd, DESTINATION_LABEL, x + 13, base + 96, left - 26, 20);
        place(hwnd, DEST, x + 19, base + 119, left - 38, 23);
        place(hwnd, HANDOFF, x + 13, base + 155, left - 26, 29);
        return;
    }

    let form_x = x + 13;
    let form_width = width - 26;
    place(hwnd, MEDIA_LABEL, form_x, base + 14, 54, 20);
    place(hwnd, MEDIA, form_x + 56, base + 9, form_width - 185, 29);
    place(hwnd, INSPECT, form_x + form_width - 121, base + 9, 121, 29);

    let disk_width = form_width * 29 / 100;
    let edition_width = form_width * 38 / 100;
    place(hwnd, DISK, form_x, base + 46, disk_width, 29);
    place(
        hwnd,
        IMAGE_INDEX,
        form_x + disk_width + 8,
        base + 46,
        edition_width,
        29,
    );
    place(
        hwnd,
        MODE,
        form_x + disk_width + edition_width + 16,
        base + 46,
        form_width - disk_width - edition_width - 16,
        29,
    );

    place(hwnd, BACKUP_LABEL, form_x, base + 87, 129, 20);
    place(
        hwnd,
        PRESERVATION,
        form_x + 130,
        base + 82,
        form_width - 130,
        29,
    );
    place(hwnd, DESTINATION_LABEL, form_x, base + 123, 59, 20);
    place(hwnd, DEST, form_x + 60, base + 118, form_width - 325, 29);
    place(hwnd, PLAN, form_x + form_width - 257, base + 118, 121, 29);
    place(hwnd, SAVE, form_x + form_width - 128, base + 118, 128, 29);

    let top = area.body_top(DEPLOY);
    let bottom = area.body_bottom(DEPLOY);
    place(
        hwnd,
        CONTENT,
        x + 12,
        top + 40,
        left - 24,
        bottom - top - 50,
    );
    place(
        hwnd,
        INSTALL,
        right + 13,
        if media_loaded(app) {
            bottom - 38
        } else {
            top + 23
        },
        right_width - 26,
        29,
    );
}

pub unsafe fn render(hwnd: HWND, app: &App) -> bool {
    let deployment = app.section == DEPLOY;
    let handoff = app.section == RESTORE;
    let media = app.deployment.source.as_ref().map_or_else(
        || "Escolher Windows para instalar".into(),
        |source| source.label(app.technician),
    );
    set(hwnd, MEDIA, &media);
    let backup = app.session.backup.as_ref().map_or_else(
        || "Escolher backup".into(),
        |backup| backup.location.label(app.technician),
    );
    set(hwnd, PRESERVATION, &backup);
    let disk = app.deployment.disk.as_ref().map_or_else(
        || "Escolher disco".into(),
        |id| {
            app.report
                .as_ref()
                .and_then(|report| {
                    report
                        .inventory
                        .disks
                        .iter()
                        .find(|disk| disk.identity == *id)
                })
                .and_then(|disk| disk.number)
                .map_or_else(
                    || "Disco indisponível".into(),
                    |number| {
                        if app.technician {
                            format!("Disco {number} · {id}")
                        } else {
                            format!("Disco {number}")
                        }
                    },
                )
        },
    );
    set(hwnd, DISK, &disk);
    for id in [
        MEDIA,
        MEDIA_LABEL,
        INSPECT,
        IMAGE_INDEX,
        DISK,
        MODE,
        PLAN,
        SAVE,
        INSTALL,
    ] {
        show(hwnd, id, deployment);
    }
    show(hwnd, BACKUP_LABEL, deployment || handoff);
    show(hwnd, PRESERVATION, deployment || handoff);
    show(hwnd, HANDOFF, handoff);
    set(
        hwnd,
        INSTALL,
        if lab_available() {
            "Instalar em laboratório"
        } else {
            "Instalar Windows"
        },
    );
    EnableWindow(control(hwnd, INSTALL), (!app.busy && lab_ready(app)) as i32);
    for id in [
        MEDIA,
        INSPECT,
        IMAGE_INDEX,
        DISK,
        MODE,
        PRESERVATION,
        HANDOFF,
    ] {
        EnableWindow(control(hwnd, id), (!app.busy) as i32);
    }
    EnableWindow(
        control(hwnd, PLAN),
        (!app.busy
            && app
                .deployment
                .media
                .as_ref()
                .is_some_and(|media| media.images.get(selection(hwnd, IMAGE_INDEX)).is_some())
            && app.deployment.disk.is_some()) as i32,
    );
    EnableWindow(
        control(hwnd, SAVE),
        (!app.busy && app.deployment.plan.is_some()) as i32,
    );

    EnableWindow(
        control(hwnd, HANDOFF),
        (!app.busy && app.session.backup.is_some() && app.session.destination(RESTORE).is_ok())
            as i32,
    );
    if deployment {
        show(hwnd, CONTENT, media_loaded(app));
        set(
            hwnd,
            CONTENT,
            &if let Some(status) = &app.deployment.lab_status {
                format!(
                    "{status}\n{}",
                    preview_text(&app.deployment, app.technician)
                )
            } else {
                preview_text(&app.deployment, app.technician)
            },
        );
    }

    deployment || handoff
}

fn preview_text(state: &Deployment, technician: bool) -> String {
    let Some(media) = &state.media else {
        return "Escolha uma imagem do Windows para consultar as edições.".into();
    };
    let mut text = String::new();

    if let Some(plan) = &state.plan {
        let label = match plan.result {
            Qualification::Supported => "Compatível para planejamento",
            Qualification::RequiresIntervention => "Requer intervenção",
            Qualification::Unsupported => "Configuração sem suporte",
            Qualification::Blocked => "Bloqueado",
        };
        let _ = writeln!(text, "\nPLANO | {label}\nDisco: {}\nFirmware: {} | Saúde: {}\nCriptografia: {}\nPartições observadas: {} | ESP: {}\nCapacidade: {:?} bytes | Mínimo: {:?} bytes\nInstalações encontradas: {} | Risco aos dados: {}",
            if technician { plan.target.disk.identity.clone() } else { plan.target.disk.number.map_or_else(|| "indisponível".into(), |number| number.to_string()) }, plan.target.firmware, plan.target.storage_health, plan.target.encryption,
            plan.target.disk.partitions.len(), plan.observed_efi_partitions.len(), plan.target.disk.capacity,
            plan.minimum_bytes, plan.target.discovery.installations.len(), if plan.existing_data_at_risk { "Sim" } else { "Não" });
        let preservation = plan.target.preservation.as_ref().map_or_else(
            || "Nenhum backup verificado nesta prévia.".to_owned(),
            |backup| {
                format!(
                    "Backup {}: {} arquivos conferidos; cobertura integral não comprovada.",
                    backup.backup_operation, backup.verified_files
                )
            },
        );
        let _ = writeln!(text, "{preservation}\nPróxima ação: {}", plan.next_action);
        for finding in &plan.findings {
            let _ = writeln!(text, "• {}", finding.reason);
        }
        for partition in &plan.intended_partitions {
            let _ = writeln!(text, "Partição planejada: {partition}");
        }

        if technician {
            let _ = writeln!(
                text,
                "\nSHA-256 do plano: {}\n{}",
                plan.fingerprint, plan.authorization_required
            );
            for volume in &plan.target.volumes {
                let _ = writeln!(
                    text,
                    "{} | {:?} | {:?}",
                    volume.identity, volume.filesystem, volume.paths
                );
            }
        }
    }

    let _ = writeln!(
        text,
        "\nMídia inspecionada\n{} · {}",
        paths::display(&media.path, technician),
        crate::resources::size(media.container_bytes),
    );

    if technician {
        let _ = writeln!(text, "SHA-256: {}", media.sha256);
    }
    for image in &media.images {
        let _ = writeln!(
            text,
            "\n{}\nEdição: {} | Arquitetura: {} | Versão: {} | Tamanho expandido: {}",
            image.name.as_deref().unwrap_or("indisponível"),
            image.edition.as_deref().unwrap_or("indisponível"),
            image.architecture.as_deref().unwrap_or("indisponível"),
            image.version.as_deref().unwrap_or("indisponível"),
            image
                .expanded_bytes
                .map(|value| format!("{value} bytes"))
                .as_deref()
                .unwrap_or("indisponível")
        );
    }

    text
}

pub fn select_disk(app: &mut App, disk: String) {
    app.deployment.disk = Some(disk);
    invalidate_plan(app);
}

pub fn invalidate_plan(app: &mut App) {
    app.deployment.plan = None;
    app.deployment.lab_status = None;
}

pub unsafe fn inspect(hwnd: HWND, app: &mut App, source: Location) {
    app.deployment.source = Some(source.clone());
    app.deployment.media = None;
    invalidate_plan(app);
    let cancel = Arc::new(AtomicBool::new(false));
    app.cancel = cancel.clone();
    let sender = app.tx.clone();
    work(app, move || {
        let result = (|| {
            let path = source.revalidate()?;
            let mut previous = 0;
            archrescue_deployment::inspect(&path, None, &cancel, |read, total| {
                if read == total || read - previous >= 32 * 1024 * 1024 {
                    let _ = sender.send(Event::Deployment(Update::Progress(read, total)));
                    previous = read;
                }
            })
        })();
        match result {
            Ok(media) => Event::Deployment(Update::Media(Box::new(media))),
            Err(error) => Event::Failure(error),
        }
    });
    app.cancellable = true;
    set(
        hwnd,
        STATUS,
        "Inspecionando os índices e o SHA-256 da imagem…",
    );
}

pub unsafe fn update(hwnd: HWND, app: &mut App, event: Update) {
    match event {
        Update::Progress(read, total) => set(
            hwnd,
            STATUS,
            &format!("SHA-256 da mídia: {read}/{total} bytes"),
        ),
        Update::Media(media) => {
            SendMessageW(control(hwnd, IMAGE_INDEX), CB_RESETCONTENT, 0, 0);
            for image in &media.images {
                add_option(
                    hwnd,
                    IMAGE_INDEX,
                    image
                        .name
                        .as_deref()
                        .or(image.edition.as_deref())
                        .unwrap_or("Edição sem nome"),
                );
            }
            SendMessageW(control(hwnd, IMAGE_INDEX), CB_SETCURSEL, 0, 0);
            app.deployment.media = Some(*media);
            set(
                hwnd,
                STATUS,
                "Mídia inspecionada. Escolha a edição, o disco e o modo.",
            );
        }
        #[cfg(all(feature = "lab-deployment", target_arch = "x86_64"))]
        Update::LabProgress(step) => {
            use archrescue_deployment::lab::windows::Step;

            let status = match step {
                Step::Preflight => "Em execução · Conferindo autorização, imagem e discos",
                Step::Partition => "Em execução · Preparando partições GPT",
                Step::ApplyImage => "Em execução · Aplicando imagem do Windows",
                Step::ConfigureBoot => "Em execução · Configurando a inicialização UEFI",
                Step::Verify => "Em execução · Verificando a implantação",
            };
            app.deployment.lab_status = Some(status.into());
            set(hwnd, STATUS, status);
        }
        #[cfg(all(feature = "lab-deployment", target_arch = "x86_64"))]
        Update::LabFinished(result) => {
            use archrescue_deployment::lab::windows::ExecutionState;

            let (summary, detail) = match *result {
                Ok(evidence) => {
                    let summary = match evidence.execution.execution_state {
                        ExecutionState::AwaitingFirstBoot => "Aguardando primeira inicialização",
                        ExecutionState::Blocked => "Implantação bloqueada",
                        ExecutionState::Cancelled => "Implantação cancelada",
                        ExecutionState::Failed | ExecutionState::Running => "Falha na implantação",
                    };
                    let errors = evidence
                        .execution
                        .errors
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("\n");
                    (
                        summary,
                        format!(
                            "{}\n{}",
                            paths::display(&evidence.path, app.technician),
                            errors
                        ),
                    )
                }
                Err(error) => ("Implantação bloqueada", error.to_string()),
            };
            app.deployment.lab_status = Some(format!("{summary}\n{detail}"));
            set(hwnd, STATUS, summary);
        }
        Update::Plan(plan) => {
            app.deployment.lab_status = None;
            app.deployment.plan = Some(*plan);
            set(hwnd, STATUS, "Prévia disponível para revisão e exportação.");
        }
    }
}

pub unsafe fn action(hwnd: HWND, app: &mut App, id: usize, notification: u32) -> bool {
    match id {
        MEDIA | INSPECT => session::request(app, picker::Purpose::Media),
        DISK => session::request(app, picker::Purpose::DeploymentDisk),
        PRESERVATION => session::request(app, picker::Purpose::Backup),
        IMAGE_INDEX | MODE if notification == CBN_SELCHANGE => invalidate_plan(app),
        PLAN => {
            let Some(media) = &app.deployment.media else {
                return true;
            };
            let Some(image) = media.images.get(selection(hwnd, IMAGE_INDEX)) else {
                return true;
            };
            let Some(disk) = app.deployment.disk.clone() else {
                set(hwnd, STATUS, "Escolha o disco de instalação.");
                return true;
            };
            let intent = [
                Intent::Inspection,
                Intent::Repair,
                Intent::KeepData,
                Intent::Clean,
            ][selection(hwnd, MODE).min(3)];
            let index = image.index;
            let Some(source) = app.deployment.source.clone() else {
                return true;
            };
            let hash = media.sha256.clone();
            let separate_lab_backup = lab_available()
                && intent == Intent::Clean
                && app.report.as_ref().is_some_and(|report| {
                    report.inventory.disks.iter().any(|candidate| {
                        candidate.identity == disk && candidate.partitions.is_empty()
                    })
                });
            let backup = if separate_lab_backup {
                None
            } else {
                app.session.backup.clone()
            };
            app.deployment.plan = None;
            let cancel = app.cancel.clone();
            cancel.store(false, Ordering::Relaxed);
            work(app, move || {
                let result = (|| {
                    let path = source.revalidate()?;
                    let backup = backup.map(|backup| backup.revalidate()).transpose()?;
                    archrescue_deployment::create_plan(
                        &path,
                        &hash,
                        index,
                        &disk,
                        intent,
                        backup.as_deref(),
                        &cancel,
                    )
                })();
                match result {
                    Ok(plan) => Event::Deployment(Update::Plan(Box::new(plan))),
                    Err(error) => Event::Failure(error),
                }
            });
            app.cancellable = true;
            set(
                hwnd,
                STATUS,
                "Revalidando mídia, identidade e pré-requisitos…",
            );
        }
        #[cfg(all(feature = "lab-deployment", target_arch = "x86_64"))]
        INSTALL => {
            if !app.busy && lab_ready(app) {
                if let Some(plan) = app.deployment.plan.clone() {
                    app.confirmation = Some(confirmation::Confirmation::Deployment(Box::new(plan)));
                }
            }
        }
        SAVE => {
            let Some(plan) = app.deployment.plan.clone() else {
                return true;
            };
            let Ok(destination) = app.session.destination(DEPLOY) else {
                session::request(app, picker::Purpose::Plan);
                return true;
            };
            let sources = app.session.sources(DEPLOY);
            work(app, move || {
                let result = (|| {
                    let directory = destination.revalidate()?;
                    crate::resources::writable(&directory, &sources, 0)?;
                    let destination_id = archrescue_storage::identity(&directory)?;
                    let inventory = archrescue_storage::inventory();
                    if inventory.disks.iter().any(|disk| {
                        disk.identity == plan.target.disk.identity
                            && disk.number.is_some_and(|number| {
                                destination_id.physical_disks.contains(&number)
                            })
                    }) {
                        return Err(Error::new(
                            ErrorKind::Validation,
                            "Escolha um destino fora do disco de instalação.",
                        ));
                    }
                    archrescue_deployment::export(&plan, &directory)
                })();
                match result {
                    Ok(path) => Event::PlanExported(path),
                    Err(error) => Event::Failure(error),
                }
            });
        }
        HANDOFF => {
            let (Some(backup), Ok(destination)) =
                (app.session.backup.clone(), app.session.destination(RESTORE))
            else {
                return true;
            };
            let sources = app.session.sources(RESTORE);
            let cancel = app.cancel.clone();
            cancel.store(false, Ordering::Relaxed);
            work(app, move || {
                let result = (|| {
                    let backup_path = backup.revalidate()?;
                    let directory = destination.revalidate()?;
                    crate::resources::writable(&directory, &sources, 0)?;
                    export_handoff(&backup_path, &directory, &backup.fingerprint, &cancel)
                })();
                match result {
                    Ok(path) => Event::HandoffExported(path),
                    Err(error) => Event::Failure(error),
                }
            });
            app.cancellable = true;
            set(hwnd, STATUS, "Relendo os arquivos do backup…");
        }
        _ => return false,
    }

    super::render(hwnd, app);
    true
}

fn export_handoff(
    backup: &Path,
    destination: &Path,
    fingerprint: &str,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    let backup = archrescue_storage::safe_existing(backup)?;
    let destination = archrescue_storage::safe_existing(destination)?;
    if destination.starts_with(&backup) {
        return Err(Error::new(
            ErrorKind::Validation,
            "Exporte a referência fora do diretório da operação de backup",
        ));
    }
    let manifest = transaction::verify_payload(&backup, cancel)?;
    if manifest.plan_fingerprint != fingerprint {
        return Err(Error::new(
            ErrorKind::Conflict,
            "O backup selecionado mudou.",
        ));
    }
    let path = destination.join(format!(
        "archrescue-restore-handoff-{}.json",
        manifest.operation_id
    ));
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "version": 1,
        "backup": backup,
        "backup_operation": manifest.operation_id,
        "manifest_version": manifest.schema_version,
        "manifest_sha256": manifest.plan_fingerprint,
        "source_hint": manifest.source_hint,
        "source_directories": manifest.directories,
        "verified_files": manifest.files.len(),
        "verified_bytes": manifest.planned_bytes,
        "executable": "archrescue-restore-agent.exe",
        "authorization_required": true,
    }))
    .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))?;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| Error::io("Criar referência para restauração", error))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| Error::io("Gravar referência para restauração", error))?;
    drop(file);

    if fs::read(&path).map_err(|error| Error::io("Reler referência para restauração", error))?
        != bytes
    {
        return Err(Error::new(
            ErrorKind::Integrity,
            "Referência gravada diverge dos dados verificados",
        ));
    }

    Ok(path)
}

pub fn lab_available() -> bool {
    #[cfg(all(feature = "lab-deployment", target_arch = "x86_64"))]
    return archrescue_deployment::lab::windows::capability_available();

    #[cfg(not(all(feature = "lab-deployment", target_arch = "x86_64")))]
    false
}

fn lab_ready(app: &App) -> bool {
    #[cfg(all(feature = "lab-deployment", target_arch = "x86_64"))]
    {
        lab_available()
            && app.session.backup.is_some()
            && app.session.destination(DEPLOY).is_ok()
            && app.deployment.plan.as_ref().is_some_and(|plan| {
                plan.result == Qualification::Supported && plan.lab_layout.is_some()
            })
    }

    #[cfg(not(all(feature = "lab-deployment", target_arch = "x86_64")))]
    {
        let _ = app;
        false
    }
}

pub fn lab_status(app: &App) -> &str {
    app.deployment.lab_status.as_deref().unwrap_or_else(|| {
        if lab_ready(app) {
            "Pronto para confirmar o alvo"
        } else if lab_available() {
            "Aguardando plano, backup e pasta de evidências"
        } else {
            "Execução indisponível nesta versão"
        }
    })
}

#[cfg(all(feature = "lab-deployment", target_arch = "x86_64"))]
pub(super) unsafe fn start_lab(hwnd: HWND, app: &mut App, plan: Plan) {
    use archrescue_deployment::lab::windows::{self, Request};

    if app
        .deployment
        .plan
        .as_ref()
        .map(|current| &current.fingerprint)
        != Some(&plan.fingerprint)
    {
        return;
    }
    let (Some(backup), Ok(destination)) =
        (app.session.backup.clone(), app.session.destination(DEPLOY))
    else {
        return;
    };

    let cancel = Arc::new(AtomicBool::new(false));
    app.cancel = cancel.clone();
    let sender = app.tx.clone();
    work(app, move || {
        let result = (|| {
            let evidence = destination.revalidate()?;
            let backup = backup.revalidate()?;
            let registration = evidence.join("registration.json");
            let signature = evidence.join("registration.sig");
            let attestation = windows::read_registration(&registration, &signature)?;
            let confirmation = attestation.confirmation();
            let request = Request {
                plan: &plan,
                registration: &registration,
                signature: &signature,
                confirmation: &confirmation,
                backup: &backup,
                evidence: &evidence,
            };

            windows::execute(&request, &cancel, |step| {
                let _ = sender.send(Event::Deployment(Update::LabProgress(step)));
            })
        })();

        Event::Deployment(Update::LabFinished(Box::new(result)))
    });
    app.cancellable = true;
    set(hwnd, STATUS, "Conferindo autorização do laboratório…");
    super::render(hwnd, app);
}
