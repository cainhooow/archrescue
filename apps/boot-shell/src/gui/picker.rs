//! Themed modal for selecting discovered storage and browsing its files.

use std::{fs, path::Path, sync::mpsc::TryRecvError};

use archrescue_core::State;
use archrescue_storage as storage;

use super::*;
use crate::{
    paths,
    resources::{self, Backup, Location, Windows},
};

const FILTER: usize = 310;
pub const LIST: usize = 311;
const DETAILS: usize = 312;
const BACK: usize = 313;
const BREADCRUMB: usize = 314;
const OPEN_FOLDER: usize = 315;
const CREATE: usize = 316;
const NAME: usize = 317;
const APPLY: usize = IDYES as usize;
const IDOK_ID: usize = IDOK as usize;
const DISMISS: usize = IDNO as usize;
const CHOICE_LIMIT: usize = 4096;
const FILTER_LABEL: usize = 318;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Source,
    Windows,
    Efi,
    SaveFiles,
    Preserve,
    Report,
    Media,
    DeploymentDisk,
    Backup,
    Handoff,
    Plan,
    ConsoleLog,
}

impl Purpose {
    pub fn title(self) -> &'static str {
        match self {
            Self::Source => "Escolher pasta de origem",
            Self::Windows => "Escolher Windows",
            Self::Efi => "Escolher partição de inicialização",
            Self::SaveFiles => "Escolher onde salvar",
            Self::Preserve => "Escolher onde preservar",
            Self::Report => "Escolher onde salvar o relatório",
            Self::Media => "Escolher Windows para instalar",
            Self::DeploymentDisk => "Escolher disco de instalação",
            Self::Backup => "Escolher backup",
            Self::Handoff => "Escolher onde salvar a referência",
            Self::Plan => "Escolher onde salvar o plano",
            Self::ConsoleLog => "Escolher onde salvar o log",
        }
    }

    fn writes(self) -> bool {
        matches!(
            self,
            Self::SaveFiles
                | Self::Preserve
                | Self::Report
                | Self::Handoff
                | Self::Plan
                | Self::ConsoleLog
        )
    }
}

#[derive(Clone)]
pub enum Selection {
    Location(Location),
    Windows(Box<Windows>),
    Backup(Backup),
    Disk(String),
}

#[derive(Clone)]
enum Target {
    Located(Location),
    Selection(Selection),
    Unavailable,
}

#[derive(Clone)]
struct Row {
    label: String,
    detail: String,
    target: Target,
}

pub struct Request {
    pub purpose: Purpose,
    rows: Vec<Row>,
    sources: Vec<Location>,
    minimum_bytes: u64,
    technician: bool,
}

impl Request {
    pub fn new(
        purpose: Purpose,
        report: &Report,
        catalog: &resources::Catalog,
        selected_windows: Option<&Windows>,
        sources: Vec<Location>,
        minimum_bytes: u64,
        technician: bool,
    ) -> Self {
        let mut request = Self {
            purpose,
            rows: Vec::new(),
            sources,
            minimum_bytes,
            technician,
        };

        match purpose {
            Purpose::Windows => {
                for windows in &catalog.windows {
                    request.rows.push(Row {
                        label: windows.label(technician),
                        detail: if catalog.windows.len() == 1 {
                            "Instalação corroborada · sugerida".into()
                        } else {
                            "Instalação corroborada · confirme o disco".into()
                        },
                        target: Target::Selection(Selection::Windows(Box::new(windows.clone()))),
                    });
                }
            }
            Purpose::Efi => {
                let association =
                    selected_windows.and_then(|windows| {
                        report.diagnosis.associations.iter().find(|association| {
                            association.installation == windows.installation.path
                        })
                    });
                if let Some(association) = association {
                    let ambiguous = association.efi_partitions.len() != 1;
                    for path in &association.mounted_efi {
                        request.rows.push(Row {
                            label: format!("Partição EFI · Disco {} · {}", association.disk_number, paths::display(path, technician)),
                            detail: if ambiguous {
                                "Mais de uma ESP no disco. A associação permanece ambígua e o plano pode ser bloqueado.".into()
                            } else {
                                "Mesmo disco GPT. Referência do BCD ainda precisa ser comprovada.".into()
                            },
                            target: catalog.efi.iter().find(|(candidate, _)| candidate == path)
                                .map_or(Target::Unavailable, |(_, location)| Target::Located(location.clone())),
                        });
                    }
                }
            }
            Purpose::DeploymentDisk => {
                for disk in &report.inventory.disks {
                    let Some(number) = disk.number else {
                        continue;
                    };
                    let unique = report
                        .inventory
                        .disks
                        .iter()
                        .filter(|candidate| candidate.identity == disk.identity)
                        .count()
                        == 1;
                    let usable =
                        unique && disk.identity.starts_with("gpt:") && disk.read_only != Some(true);
                    let details = if technician {
                        format!(
                            "{} · {}",
                            disk.style.as_deref().unwrap_or("Layout indisponível"),
                            disk.identity
                        )
                    } else {
                        disk.style
                            .clone()
                            .unwrap_or_else(|| "Layout indisponível".into())
                    };
                    request.rows.push(Row {
                        label: format!(
                            "Disco {number} · {}",
                            disk.capacity
                                .map_or_else(|| "Capacidade indisponível".into(), resources::size)
                        ),
                        detail: if usable {
                            details
                        } else {
                            format!("{details} · Requer GPT com identidade única e disco gravável.")
                        },
                        target: if usable {
                            Target::Selection(Selection::Disk(disk.identity.clone()))
                        } else {
                            Target::Unavailable
                        },
                    });
                }
            }
            _ => {
                if purpose == Purpose::Backup {
                    request
                        .rows
                        .extend(catalog.backups.iter().map(|(entry, location)| Row {
                            label: backup_label(entry, technician),
                            detail: backup_status(entry),
                            target: if entry.error.is_none() {
                                let manifest =
                                    entry.manifest.as_ref().expect("validated checkpoint");
                                Target::Selection(Selection::Backup(Backup {
                                    location: location.clone(),
                                    operation: manifest.operation_id.clone(),
                                    fingerprint: manifest.plan_fingerprint.clone(),
                                }))
                            } else {
                                Target::Unavailable
                            },
                        }));
                } else if purpose == Purpose::Media {
                    request.rows.extend(catalog.media.iter().map(|path| Row {
                        label: path.label(technician),
                        detail: "Imagem encontrada. A inspeção verifica cabeçalho, edições e SHA-256.".into(),
                        target: Target::Located(path.clone()),
                    }));
                }

                let mut volumes = report.inventory.volumes.iter().collect::<Vec<_>>();
                if purpose.writes() {
                    volumes.sort_by_key(|volume| {
                        !report.inventory.disks.iter().any(|disk| {
                            disk.number
                                .is_some_and(|number| volume.physical_disks.contains(&number))
                                && (disk.removable == Some(true) || disk.bus == Some(7))
                        })
                    });
                }
                for volume in volumes {
                    let path = catalog
                        .volumes
                        .iter()
                        .find(|(id, _)| id == &volume.identity)
                        .map(|(_, path)| path.clone());
                    let blocked = purpose.writes()
                        && report.inventory.disks.iter().any(|disk| {
                            disk.read_only == Some(true)
                                && disk
                                    .number
                                    .is_some_and(|number| volume.physical_disks.contains(&number))
                        });
                    let detail = if blocked {
                        "Disco somente leitura.".into()
                    } else if path.is_none() {
                        "Sistema de arquivos inacessível.".into()
                    } else {
                        "Selecione ou abra para consultar as pastas.".into()
                    };

                    request.rows.push(Row {
                        label: resources::volume_description(volume, &report.inventory, technician),
                        detail,
                        target: if blocked {
                            Target::Unavailable
                        } else {
                            path.map_or(Target::Unavailable, Target::Located)
                        },
                    });
                }
            }
        }

        request
    }

    fn choose(&self, target: Target) -> Result<Selection> {
        match target {
            Target::Selection(Selection::Windows(windows)) => {
                windows.revalidate()?;
                Ok(Selection::Windows(windows))
            }
            Target::Selection(Selection::Disk(id)) => {
                let inventory = storage::inventory();
                let mut matches = inventory.disks.iter().filter(|disk| disk.identity == id);
                let disk = matches
                    .next()
                    .filter(|_| matches.next().is_none())
                    .ok_or_else(|| {
                        unavailable("O disco foi removido ou tem identidade ambígua.")
                    })?;
                storage::windows::require_writable_disk(&disk.device_path)?;

                Ok(Selection::Disk(id))
            }
            Target::Selection(Selection::Backup(backup)) => {
                backup.revalidate()?;
                Ok(Selection::Backup(backup))
            }
            Target::Located(location) => {
                location.revalidate()?;
                self.choose_location(location)
            }
            _ => Err(unavailable("Recurso indisponível.")),
        }
    }

    fn choose_location(&self, location: Location) -> Result<Selection> {
        match self.purpose {
            Purpose::Backup => {
                let checkpoint = transaction::checkpoint(&location.path);
                if let Some(error) = checkpoint.error {
                    return Err(error);
                }
                let manifest = checkpoint
                    .manifest
                    .ok_or_else(|| unavailable("Escolha a pasta de uma operação ArchRescue."))?;
                Ok(Selection::Backup(Backup {
                    location,
                    operation: manifest.operation_id,
                    fingerprint: manifest.plan_fingerprint,
                }))
            }
            Purpose::Media if !location.path.is_file() || !image_file(&location.path) => Err(
                unavailable("Escolha WIM ou ESD. Para ISO, use mídia montada externamente."),
            ),
            Purpose::Media => Ok(Selection::Location(location)),
            _ if location.path.is_dir() => {
                if self.purpose.writes() {
                    resources::writable(&location.path, &self.sources, self.minimum_bytes)?;
                }
                Ok(Selection::Location(location))
            }
            _ => Err(unavailable("Escolha uma pasta.")),
        }
    }

    fn browse(&self, path: &Path) -> Result<(Location, Vec<Row>)> {
        let location = Location::capture(path)?;
        if self.purpose.writes() {
            resources::writable(&location.path, &self.sources, self.minimum_bytes)?;
        }
        let mut rows = Vec::new();
        let children =
            fs::read_dir(&location.path).map_err(|error| Error::io("Abrir pasta", error))?;
        for child in children.take(CHOICE_LIMIT) {
            let child = child.map_err(|error| Error::io("Ler pasta", error))?;
            let label = child.file_name().to_string_lossy().into_owned();
            let kind = child
                .file_type()
                .map_err(|error| Error::io("Ler tipo", error))?;
            if kind.is_dir() {
                let child_path = child.path();
                let access = Location::capture(&child_path);
                let (target, detail) = match access {
                    Ok(child) if child.same_partition(&location) => {
                        (Target::Located(child), "Pasta".into())
                    }
                    Ok(_) => (
                        Target::Unavailable,
                        "A pasta pertence a outro volume.".into(),
                    ),
                    Err(error) => (Target::Unavailable, error_message(&error)),
                };
                rows.push(Row {
                    label,
                    detail,
                    target,
                });
            } else if self.purpose == Purpose::Media && image_file(&child.path()) {
                let (target, detail) = match Location::capture(&child.path()) {
                    Ok(child) if child.same_partition(&location) => {
                        (Target::Located(child), "Imagem do Windows".into())
                    }
                    Ok(_) => (
                        Target::Unavailable,
                        "A imagem pertence a outro volume.".into(),
                    ),
                    Err(error) => (Target::Unavailable, error_message(&error)),
                };
                rows.push(Row {
                    label,
                    detail,
                    target,
                });
            }
        }
        rows.sort_by_key(|row| row.label.to_lowercase());

        Ok((location, rows))
    }

    fn create_folder(&self, parent: &Location, name: &str) -> Result<(Location, Vec<Row>)> {
        let name = Path::new(name);
        if !self.purpose.writes()
            || name.components().count() != 1
            || name.as_os_str().to_string_lossy().contains(['/', '\\'])
        {
            return Err(unavailable("Informe um nome de pasta, sem caminho."));
        }
        archrescue_backup::portable_relative(name)?;
        let parent = parent.revalidate()?;
        resources::writable(&parent, &self.sources, self.minimum_bytes)?;

        let path = parent.join(name);
        fs::create_dir(&path).map_err(|error| Error::io("Criar pasta", error))?;

        self.browse(&path)
    }
}

fn image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["wim", "esd"]
                .iter()
                .any(|kind| extension.eq_ignore_ascii_case(kind))
        })
}

fn backup_label(entry: &CheckpointEntry, technician: bool) -> String {
    let path = paths::display(&entry.directory, technician);
    entry.manifest.as_ref().map_or(path.clone(), |manifest| {
        format!(
            "Backup ArchRescue · {} arquivos · {} · {path}",
            manifest.files.len(),
            resources::size(manifest.planned_bytes)
        )
    })
}

fn backup_status(entry: &CheckpointEntry) -> String {
    if let Some(error) = &entry.error {
        return error_message(error);
    }

    let recorded = match entry.status.as_ref().map(|status| status.state) {
        Some(State::Completed) => "Conclusão registrada",
        Some(State::Running | State::Cancelled | State::Pending) | None => {
            "Checkpoint · retomada sujeita à validação"
        }
        Some(state) => state_label(state),
    };
    format!("{recorded} · conteúdo ainda precisa ser relido.")
}

fn unavailable(message: &str) -> Error {
    Error::new(ErrorKind::Validation, message)
}

struct Model {
    location: Option<Location>,
    rows: Vec<Row>,
    visible: Vec<usize>,
    history: Vec<Option<Location>>,
    receiver: Option<Receiver<Update>>,
    result: Option<Selection>,
}

enum Update {
    Open(Result<(Location, Vec<Row>)>, Direction),
    Choose(Result<Selection>),
}

#[derive(Clone, Copy)]
enum Direction {
    Forward,
    Back,
}

struct Dialog<'a> {
    owner: &'a WindowState,
    request: Arc<Request>,
    model: RefCell<Model>,
    frame: RefCell<renderer::Frame>,
}

pub unsafe fn show(hwnd: HWND, state: &WindowState, request: Request) -> Option<Selection> {
    #[repr(C, align(4))]
    struct Template([u16; 12]);

    let style = WS_POPUP | WS_CLIPCHILDREN;
    let template = Template([
        style as u16,
        (style >> 16) as u16,
        0,
        0,
        0,
        0,
        0,
        1,
        1,
        0,
        0,
        0,
    ]);
    let dialog = Dialog {
        owner: state,
        model: RefCell::new(Model {
            location: None,
            rows: request.rows.clone(),
            visible: Vec::new(),
            history: Vec::new(),
            receiver: None,
            result: None,
        }),
        request: Arc::new(request),
        frame: RefCell::default(),
    };
    let focus = GetFocus();
    state.confirming.set(true);

    // SAFETY: DialogBox owns the child windows and returns before these borrows expire.
    let result = DialogBoxIndirectParamW(
        GetModuleHandleW(null()),
        template.0.as_ptr().cast(),
        hwnd,
        Some(procedure),
        &dialog as *const Dialog<'_> as LPARAM,
    );

    state.confirming.set(false);
    if IsWindowVisible(focus) != 0 && IsWindowEnabled(focus) != 0 {
        SetFocus(focus);
    }
    if result == -1 {
        set(
            hwnd,
            STATUS,
            "Não foi possível abrir o seletor de recursos.",
        );
    }

    dialog.model.into_inner().result
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        let dialog = &*(lp as *const Dialog<'_>);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, lp);
        if let Err(error) = initialize(hwnd, dialog) {
            eprintln!("{error}");
            EndDialog(hwnd, 0);
        }
        return 0;
    }
    let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog<'_>;
    let Some(dialog) = dialog.as_ref() else {
        return 0;
    };

    match message {
        WM_CLOSE | WM_DISPLAYCHANGE => {
            if dialog
                .model
                .try_borrow()
                .is_ok_and(|model| model.receiver.is_none())
            {
                EndDialog(hwnd, 0);
            }
            1
        }
        WM_ERASEBKGND => 1,
        WM_MEASUREITEM if lp != 0 => {
            // Win32 supplies writable item geometry during child creation.
            (*(lp as *mut MEASUREITEMSTRUCT)).itemHeight = 30;
            1
        }
        WM_DRAWITEM if lp != 0 => {
            let item = &*(lp as *const DRAWITEMSTRUCT);
            if let Ok(mut frame) = dialog.owner.control_frame.try_borrow_mut() {
                presentation::control_frame(
                    &mut frame,
                    item,
                    &dialog.owner.theme,
                    DASHBOARD,
                    false,
                );
            }
            1
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT | WM_CTLCOLORLISTBOX => {
            let color = if message == WM_CTLCOLOREDIT {
                theme::INPUT
            } else {
                theme::MODAL
            };
            SetTextColor(wp as HDC, theme::TEXT);
            SetBkColor(wp as HDC, color);
            SetDCBrushColor(wp as HDC, color);
            GetStockObject(DC_BRUSH) as isize
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut paint);
            dialog
                .frame
                .borrow()
                .present(dc, paint.rcPaint, paint.rcPaint.left, paint.rcPaint.top);
            EndPaint(hwnd, &paint);
            1
        }
        WM_TIMER | WM_COMMAND => {
            let Ok(mut model) = dialog.model.try_borrow_mut() else {
                return 0;
            };
            if message == WM_TIMER {
                receive(hwnd, dialog, &mut model);
            } else {
                command(hwnd, dialog, &mut model, wp & 0xffff, (wp >> 16) as u32);
            }
            scroll::redraw(hwnd, LIST);
            scroll::redraw(hwnd, DETAILS);
            1
        }
        _ => 0,
    }
}

unsafe fn initialize(hwnd: HWND, dialog: &Dialog<'_>) -> Result<()> {
    let owner = GetWindow(hwnd, GW_OWNER);
    let area = layout::Layout::read(owner);
    let theme = &dialog.owner.theme;
    let mut origin = POINT::default();
    ClientToScreen(owner, &mut origin);

    let width = (area.width - 40).min(875);
    let height = (area.height - 40).min(570);
    let left = (area.width - width) / 2;
    let top = (area.height - height) / 2;
    let card = RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    let x = left + 22;
    let content = width - 44;

    if SetWindowPos(
        hwnd,
        null_mut(),
        origin.x,
        origin.y,
        area.width,
        area.height,
        SWP_NOZORDER | SWP_NOACTIVATE,
    ) == 0
    {
        return Err(os_error("Position resource picker"));
    }

    let screen = GetDC(null_mut());
    if screen.is_null() {
        return Err(os_error("Read picker backdrop"));
    }
    let mut frame = dialog.frame.borrow_mut();
    let ready = frame.reserve(screen, area.width, area.height);
    let captured = ready && frame.dim_backdrop(screen, origin, area.width, area.height);
    ReleaseDC(null_mut(), screen);
    if !ready {
        return Err(os_error("Allocate picker framebuffer"));
    }

    frame.draw(area.width, area.height, |scene| {
        if !captured {
            scene.clear(theme::BACKGROUND);
        }
        scene.surface(card, theme::MODAL, theme::MODAL_BORDER, 13);
        scene.text(
            theme.brand(),
            RECT {
                left: x,
                right: x + content,
                top: top + 15,
                bottom: top + 50,
            },
            dialog.request.purpose.title(),
            theme::TEXT,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        );
    });
    drop(frame);

    for (id, label) in [
        (BACK, "Voltar"),
        (OPEN_FOLDER, "Abrir pasta"),
        (DISMISS, "Cancelar"),
        (APPLY, "Confirmar"),
        (CREATE, "Criar pasta"),
    ] {
        add(hwnd, id, "BUTTON", label, WS_TABSTOP, theme.body())?;
    }
    for (id, hint) in [(FILTER, "Filtrar recursos"), (NAME, "Nome da nova pasta")] {
        add(
            hwnd,
            id,
            "EDIT",
            "",
            WS_TABSTOP | ES_AUTOHSCROLL as u32,
            theme.body(),
        )?;
        SendMessageW(control(hwnd, id), 0x1501, 1, wide(hint).as_ptr() as LPARAM);
    }
    add(
        hwnd,
        BREADCRUMB,
        "EDIT",
        "",
        ES_READONLY as u32 | ES_AUTOHSCROLL as u32,
        theme.body(),
    )?;
    add(hwnd, FILTER_LABEL, "STATIC", "Filtrar", 0, theme.body())?;
    add(
        hwnd,
        DETAILS,
        "EDIT",
        "",
        WS_TABSTOP | ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL,
        theme.body(),
    )?;
    add(
        hwnd,
        LIST,
        "LISTBOX",
        "",
        WS_TABSTOP | WS_VSCROLL | LBS_NOTIFY as u32,
        theme.body(),
    )?;
    scroll::attach_for(hwnd, LIST, control(hwnd, LIST), owner)?;
    scroll::attach_for(hwnd, DETAILS, control(hwnd, DETAILS), owner)?;

    place(hwnd, BACK, x, top + 56, 76, 28);
    place(hwnd, BREADCRUMB, x + 84, top + 58, content - 84, 26);
    place(hwnd, FILTER_LABEL, x, top + 96, 52, 22);
    place(hwnd, FILTER, x + 55, top + 93, content - 55, 26);
    place(hwnd, LIST, x, top + 130, content, height - 278);
    place(hwnd, DETAILS, x, card.bottom - 142, content, 55);
    place(hwnd, NAME, x, card.bottom - 78, content - 126, 25);
    place(hwnd, CREATE, x + content - 118, card.bottom - 80, 118, 28);
    place(hwnd, OPEN_FOLDER, x, card.bottom - 44, 112, 31);
    place(hwnd, DISMISS, x + content - 226, card.bottom - 44, 108, 31);
    place(hwnd, APPLY, x + content - 110, card.bottom - 44, 110, 31);

    refresh(hwnd, dialog, &mut dialog.model.borrow_mut());
    scroll::show(hwnd, LIST, true);
    scroll::show(hwnd, DETAILS, true);
    SendMessageW(hwnd, DM_SETDEFID, IDOK as usize, 0);
    SetFocus(control(hwnd, LIST));
    if SetTimer(hwnd, 1, 60, None) == 0 {
        return Err(os_error("Start picker event timer"));
    }

    Ok(())
}

unsafe fn refresh(hwnd: HWND, dialog: &Dialog<'_>, model: &mut Model) {
    let query = get(hwnd, FILTER).to_lowercase();
    let list = control(hwnd, LIST);
    SendMessageW(list, LB_RESETCONTENT, 0, 0);
    model.visible.clear();
    for (index, row) in model.rows.iter().enumerate() {
        if row.label.to_lowercase().contains(&query)
            && SendMessageW(list, LB_ADDSTRING, 0, wide(&row.label).as_ptr() as LPARAM) >= 0
        {
            model.visible.push(index);
        }
    }
    let title = model.location.as_ref().map_or_else(
        || "Recursos encontrados".into(),
        |location| location.label(dialog.request.technician),
    );
    set(hwnd, BREADCRUMB, &title);
    set(
        hwnd,
        DETAILS,
        if model.visible.is_empty() {
            "Nenhum recurso nesta lista. Use Voltar ou altere o filtro."
        } else if dialog.request.purpose == Purpose::Media {
            "Escolha uma imagem WIM/ESD ou abra um volume. Arquivos ISO exigem mídia montada."
        } else {
            "Selecione um recurso para consultar os detalhes."
        },
    );
    controls(hwnd, dialog, model);
}

unsafe fn selected(hwnd: HWND, model: &Model) -> Option<&Row> {
    let index = SendMessageW(control(hwnd, LIST), LB_GETCURSEL, 0, 0) as usize;
    model
        .visible
        .get(index)
        .and_then(|index| model.rows.get(*index))
}

unsafe fn controls(hwnd: HWND, dialog: &Dialog<'_>, model: &Model) {
    let busy = model.receiver.is_some();
    let row = selected(hwnd, model);
    let target = row.map(|row| &row.target);
    let browsable = !matches!(
        dialog.request.purpose,
        Purpose::Windows | Purpose::DeploymentDisk | Purpose::Efi
    );
    for (id, enabled) in [
        (DISMISS, true),
        (FILTER, true),
        (LIST, true),
        (BACK, !model.history.is_empty()),
        (
            OPEN_FOLDER,
            browsable
                && matches!(target, Some(Target::Located(location)) if !(dialog.request.purpose == Purpose::Media && image_file(&location.path))),
        ),
        (
            APPLY,
            matches!(target, Some(Target::Located(_) | Target::Selection(_)))
                || (target.is_none() && model.location.is_some()),
        ),
        (CREATE, model.location.is_some()),
        (NAME, model.location.is_some()),
    ] {
        EnableWindow(control(hwnd, id), (!busy && enabled) as i32);
    }
    for id in [CREATE, NAME] {
        super::show(hwnd, id, dialog.request.purpose.writes());
    }
}

unsafe fn command(
    hwnd: HWND,
    dialog: &Dialog<'_>,
    model: &mut Model,
    id: usize,
    notification: u32,
) {
    if model.receiver.is_some() {
        return;
    }
    let request = Arc::clone(&dialog.request);
    let row = selected(hwnd, model).cloned();
    match id {
        id if id == DISMISS || id == IDCANCEL as usize => {
            EndDialog(hwnd, 0);
        }
        IDOK_ID => {
            if GetFocus() == control(hwnd, LIST) {
                command(hwnd, dialog, model, LIST, LBN_DBLCLK);
            }
        }
        FILTER if notification == EN_CHANGE => refresh(hwnd, dialog, model),
        LIST if notification == LBN_SELCHANGE => {
            if let Some(row) = row {
                set(hwnd, DETAILS, &row.detail);
            }
            controls(hwnd, dialog, model);
        }
        BACK => match model.history.last().cloned() {
            Some(Some(location)) => start(hwnd, dialog, model, move || {
                Update::Open(
                    location.revalidate().and_then(|path| request.browse(&path)),
                    Direction::Back,
                )
            }),
            Some(None) => {
                model.history.pop();
                model.location = None;
                model.rows.clone_from(&request.rows);
                refresh(hwnd, dialog, model);
            }
            None => {}
        },
        OPEN_FOLDER | LIST if id == OPEN_FOLDER || notification == LBN_DBLCLK => {
            if let Some(row) = row {
                match row.target {
                    Target::Located(location)
                        if !matches!(request.purpose, Purpose::Efi)
                            && !(request.purpose == Purpose::Media
                                && image_file(&location.path)) =>
                    {
                        start(hwnd, dialog, model, move || {
                            Update::Open(
                                location.revalidate().and_then(|path| request.browse(&path)),
                                Direction::Forward,
                            )
                        });
                    }
                    target if notification == LBN_DBLCLK => start(hwnd, dialog, model, move || {
                        Update::Choose(request.choose(target))
                    }),
                    _ => {}
                }
            }
        }
        CREATE => {
            if let Some(parent) = model.location.clone() {
                let name = get(hwnd, NAME);
                start(hwnd, dialog, model, move || {
                    Update::Open(request.create_folder(&parent, &name), Direction::Forward)
                });
            }
        }
        APPLY => {
            let target = row.map(|row| row.target).or_else(|| {
                model
                    .location
                    .as_ref()
                    .map(|location| Target::Located(location.clone()))
            });
            if let Some(target) = target {
                let parent = model.location.clone();
                start(hwnd, dialog, model, move || {
                    Update::Choose((|| {
                        if let Some(parent) = parent {
                            parent.revalidate()?;
                        }
                        request.choose(target)
                    })())
                });
            }
        }
        _ => {}
    }
}

unsafe fn start(
    hwnd: HWND,
    dialog: &Dialog<'_>,
    model: &mut Model,
    work: impl FnOnce() -> Update + Send + 'static,
) {
    let (tx, rx) = mpsc::channel();
    match thread::Builder::new().spawn(move || {
        let _ = tx.send(work());
    }) {
        Ok(_) => {
            model.receiver = Some(rx);
            set(hwnd, DETAILS, "Validando recurso…");
            controls(hwnd, dialog, model);
        }
        Err(error) => set(
            hwnd,
            DETAILS,
            &format!("Falha ao iniciar consulta: {error}"),
        ),
    }
}

unsafe fn receive(hwnd: HWND, dialog: &Dialog<'_>, model: &mut Model) {
    let Some(receiver) = &model.receiver else {
        return;
    };
    let update = match receiver.try_recv() {
        Ok(update) => update,
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => {
            Update::Choose(Err(unavailable("A consulta foi interrompida.")))
        }
    };
    model.receiver = None;

    match update {
        Update::Open(Ok((location, rows)), direction) => {
            match direction {
                Direction::Forward => model.history.push(model.location.take()),
                Direction::Back => {
                    model.history.pop();
                }
            }
            model.location = Some(location);
            model.rows = rows;
            set(hwnd, FILTER, "");
            set(hwnd, NAME, "");
            refresh(hwnd, dialog, model);
            SetFocus(control(hwnd, LIST));
        }
        Update::Choose(Ok(selection)) => {
            model.result = Some(selection);
            EndDialog(hwnd, 1);
        }
        Update::Open(Err(error), _) | Update::Choose(Err(error)) => {
            set(hwnd, DETAILS, &error_message(&error));
            controls(hwnd, dialog, model);
            SetFocus(control(hwnd, LIST));
        }
    }
}
