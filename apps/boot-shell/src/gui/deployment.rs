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

#[derive(Default)]
pub struct Deployment {
    media: Option<Media>,
    disks: Vec<String>,
    plan: Option<Plan>,
}

pub enum Update {
    Progress(u64, u64),
    Media(Box<Media>),
    Plan(Box<Plan>),
}

pub unsafe fn controls(hwnd: HWND, font: HFONT) -> Result<()> {
    for (id, hint) in [
        (MEDIA, "Imagem WIM/ESD ou raiz da mídia montada"),
        (
            PRESERVATION,
            "Backup independente (pasta da operação, opcional para inspeção)",
        ),
    ] {
        add(
            hwnd,
            id,
            "EDIT",
            "",
            WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            font,
        )?;
        SendMessageW(control(hwnd, id), 0x1501, 1, wide(hint).as_ptr() as isize);
    }

    for id in [IMAGE_INDEX, DISK, MODE] {
        add(
            hwnd,
            id,
            "COMBOBOX",
            "",
            WS_TABSTOP | WS_VSCROLL | CBS_DROPDOWNLIST as u32,
            font,
        )?;
    }
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
        (INSPECT, "&Inspecionar"),
        (PLAN, "&Gerar prévia"),
        (SAVE, "Exportar &plano"),
        (HANDOFF, "Verificar e exportar referência"),
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

pub fn primary(id: usize) -> bool {
    matches!(id, INSPECT | PLAN | HANDOFF)
}

pub unsafe fn layout(hwnd: HWND, area: &super::layout::Layout, section: usize) {
    let x = area.left();
    let width = area.body_width();
    place(hwnd, MEDIA, x, 108, width - 137, 30);
    place(hwnd, INSPECT, x + width - 129, 108, 129, 30);
    place(hwnd, IMAGE_INDEX, x, 145, 144, 250);
    place(hwnd, DISK, x + 152, 145, width - 152, 250);
    place(hwnd, MODE, x, 184, 211, 180);
    if section == RESTORE {
        place(hwnd, PRESERVATION, x, 108, width, 30);
    } else {
        place(hwnd, PRESERVATION, x + 219, 184, width - 219, 30);
    }
    place(hwnd, PLAN, x, area.actions(), 155, 32);
    place(hwnd, SAVE, x + 163, area.actions(), 157, 32);
    place(hwnd, HANDOFF, x, area.actions(), 270, 32);
}

pub unsafe fn render(hwnd: HWND, app: &App) -> bool {
    let deployment = app.section == DEPLOY;
    let handoff = app.section == RESTORE;
    for id in [MEDIA, INSPECT, IMAGE_INDEX, DISK, MODE, PLAN, SAVE] {
        show(hwnd, id, deployment);
    }
    show(hwnd, PRESERVATION, deployment || handoff);
    show(hwnd, HANDOFF, handoff);
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
        (!app.busy && app.deployment.media.is_some() && !app.deployment.disks.is_empty()) as i32,
    );
    EnableWindow(
        control(hwnd, SAVE),
        (!app.busy && app.deployment.plan.is_some()) as i32,
    );

    if deployment {
        set(
            hwnd,
            CONTENT,
            &preview_text(&app.deployment, app.technician),
        );
    }
    if handoff {
        set(hwnd, CONTENT, "RESTAURAR ARQUIVOS APÓS REINSTALAR\n\nInforme a pasta da operação de backup e o diretório para exportar a referência. A verificação recalcula o SHA-256 dos arquivos.\n\nNo Windows instalado, execute archrescue-restore-agent.exe com a conta de destino. Use perfil, revise o mapeamento com previa e confirme com restaurar.\n\nA referência registra o manifesto e a seleção de origem. Ela não contém os arquivos nem concede autorização para restaurar.");
    }

    deployment || handoff
}

fn preview_text(state: &Deployment, technician: bool) -> String {
    let mut text = String::from(
        "PLANEJAR IMPLANTAÇÃO\n\nA execução da instalação está indisponível nesta versão.\n",
    );
    let Some(media) = &state.media else {
        text.push_str("\nInforme um WIM, ESD ou a raiz de uma mídia montada. Depois da inspeção, selecione o índice, o disco GPT e o modo.");
        return text;
    };

    let _ = writeln!(
        text,
        "\n{} | {:?} | {} bytes\nSHA-256: {}",
        media.path.display(),
        media.image_type,
        media.container_bytes,
        media.sha256
    );

    if let Some(plan) = &state.plan {
        let label = match plan.result {
            Qualification::Supported => "Compatível para planejamento",
            Qualification::RequiresIntervention => "Requer intervenção",
            Qualification::Unsupported => "Configuração sem suporte",
            Qualification::Blocked => "Bloqueado",
        };
        let _ = writeln!(text, "\nPLANO | {label}\nDisco: {}\nFirmware: {} | Saúde: {}\nCriptografia: {}\nPartições observadas: {} | ESP: {}\nCapacidade: {:?} bytes | Mínimo: {:?} bytes\nInstalações encontradas: {} | Risco aos dados: {}",
            plan.target.disk.identity, plan.target.firmware, plan.target.storage_health, plan.target.encryption,
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

    for image in &media.images {
        let _ = writeln!(
            text,
            "\nÍndice {} | {}\nEdição: {} | Arquitetura: {} | Versão: {} | Tamanho expandido: {}",
            image.index,
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

pub unsafe fn refresh_disks(hwnd: HWND, app: &mut App, report: &Report) {
    let previous = app.deployment.disks.get(selection(hwnd, DISK)).cloned();
    app.deployment.disks.clear();
    app.deployment.plan = None;
    SendMessageW(control(hwnd, DISK), CB_RESETCONTENT, 0, 0);
    for disk in &report.inventory.disks {
        if let Some(number) = disk.number.filter(|_| disk.identity.starts_with("gpt:")) {
            app.deployment.disks.push(disk.identity.clone());
            add_option(hwnd, DISK, &format!("Disco {number} | {}", disk.identity));
        }
    }

    if let Some(index) = app
        .deployment
        .disks
        .iter()
        .position(|id| Some(id) == previous.as_ref())
    {
        SendMessageW(control(hwnd, DISK), CB_SETCURSEL, index, 0);
    }
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
                add_option(hwnd, IMAGE_INDEX, &format!("Índice {}", image.index));
            }
            SendMessageW(control(hwnd, IMAGE_INDEX), CB_SETCURSEL, 0, 0);
            app.deployment.media = Some(*media);
            set(
                hwnd,
                STATUS,
                "Mídia inspecionada. Selecione o disco, índice e modo.",
            );
        }
        Update::Plan(plan) => {
            app.deployment.plan = Some(*plan);
            set(hwnd, STATUS, "Prévia disponível para revisão e exportação.");
        }
    }
}

pub unsafe fn action(hwnd: HWND, app: &mut App, id: usize, notification: u32) -> bool {
    match id {
        MEDIA if notification == EN_CHANGE => {
            app.deployment.media = None;
            app.deployment.plan = None;
        }
        IMAGE_INDEX | DISK | MODE if notification == CBN_SELCHANGE => app.deployment.plan = None,
        PRESERVATION if notification == EN_CHANGE => app.deployment.plan = None,
        INSPECT => {
            let path = PathBuf::from(get(hwnd, MEDIA));
            app.deployment.media = None;
            app.deployment.plan = None;
            let cancel = app.cancel.clone();
            let sender = app.tx.clone();
            cancel.store(false, Ordering::Relaxed);
            work(app, move || {
                let mut previous = 0;
                match archrescue_deployment::inspect(&path, None, &cancel, |read, total| {
                    if read == total || read - previous >= 32 * 1024 * 1024 {
                        let _ = sender.send(Event::Deployment(Update::Progress(read, total)));
                        previous = read;
                    }
                }) {
                    Ok(media) => Event::Deployment(Update::Media(Box::new(media))),
                    Err(error) => Event::Failure(error),
                }
            });
            app.cancellable = true;
            set(hwnd, STATUS, "Inspecionando os índices da mídia…");
        }
        PLAN => {
            let Some(media) = &app.deployment.media else {
                return true;
            };
            let Some(image) = media.images.get(selection(hwnd, IMAGE_INDEX)) else {
                return true;
            };
            let Some(disk) = app.deployment.disks.get(selection(hwnd, DISK)).cloned() else {
                set(
                    hwnd,
                    STATUS,
                    "Selecione o disco GPT pela identidade exibida.",
                );
                return true;
            };
            let intent = [
                Intent::Inspection,
                Intent::Repair,
                Intent::KeepData,
                Intent::Clean,
            ][selection(hwnd, MODE).min(3)];
            let index = image.index;
            let path = media.path.clone();
            let hash = media.sha256.clone();
            let backup = get(hwnd, PRESERVATION);
            app.deployment.plan = None;
            let cancel = app.cancel.clone();
            cancel.store(false, Ordering::Relaxed);
            work(app, move || {
                match archrescue_deployment::create_plan(
                    &path,
                    &hash,
                    index,
                    &disk,
                    intent,
                    (!backup.is_empty()).then(|| Path::new(&backup)),
                    &cancel,
                ) {
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
        SAVE => {
            let Some(plan) = app.deployment.plan.clone() else {
                return true;
            };
            let directory = PathBuf::from(get(hwnd, DEST));
            work(app, move || {
                match archrescue_deployment::export(&plan, &directory) {
                    Ok(path) => Event::PlanExported(path),
                    Err(error) => Event::Failure(error),
                }
            });
        }
        HANDOFF => {
            let backup = PathBuf::from(get(hwnd, PRESERVATION));
            let directory = PathBuf::from(get(hwnd, DEST));
            let cancel = app.cancel.clone();
            cancel.store(false, Ordering::Relaxed);
            work(app, move || {
                match export_handoff(&backup, &directory, &cancel) {
                    Ok(path) => Event::PlanExported(path),
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

fn export_handoff(backup: &Path, destination: &Path, cancel: &AtomicBool) -> Result<PathBuf> {
    let backup = archrescue_storage::safe_existing(backup)?;
    let destination = archrescue_storage::safe_existing(destination)?;
    if destination.starts_with(&backup) {
        return Err(Error::new(
            ErrorKind::Validation,
            "Exporte a referência fora do diretório da operação de backup",
        ));
    }
    let manifest = transaction::verify_payload(&backup, cancel)?;
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
