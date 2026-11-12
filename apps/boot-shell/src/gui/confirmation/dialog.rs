//! Reference confirmation card hosted by a Win32 modal dialog.

use super::*;
use crate::gui::{renderer::Frame, theme::*};

const FIRST_PATH: usize = 300;
const SECOND_PATH: usize = 301;

pub(super) struct Prompt {
    pub title: &'static str,
    pub question: String,
    pub paths: [(&'static str, String); 2],
    pub notice: (&'static str, &'static str),
    pub width: i32,
}

struct Dialog<'a> {
    prompt: &'a Prompt,
    state: &'a WindowState,
    frame: RefCell<Frame>,
}

pub(super) unsafe fn show(owner: HWND, state: &WindowState, prompt: &Prompt) -> bool {
    #[repr(C, align(4))]
    struct Template {
        // DLGTEMPLATE is followed by three empty WORDs (menu, class, and title).
        words: [u16; 12],
    }

    let style = WS_POPUP;
    let template = Template {
        words: [
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
        ],
    };
    let dialog = Dialog {
        prompt,
        state,
        frame: RefCell::default(),
    };
    let previous_focus = GetFocus();

    // The modal loop owns its HWNDs. The borrowed data and template stay live
    // until it returns, and the caller has released the application borrow.
    let result = DialogBoxIndirectParamW(
        GetModuleHandleW(null()),
        template.words.as_ptr().cast(),
        owner,
        Some(procedure),
        &dialog as *const Dialog<'_> as LPARAM,
    );

    if IsWindowVisible(previous_focus) != 0 && IsWindowEnabled(previous_focus) != 0 {
        SetFocus(previous_focus);
    }
    if result == -1 {
        eprintln!("{}", os_error("Open confirmation dialog"));
    }

    result == IDYES as isize
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if message == WM_INITDIALOG {
        let dialog = &*(lp as *const Dialog<'_>);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, lp);
        if let Err(error) = initialize(hwnd, dialog) {
            eprintln!("{error}");
            EndDialog(hwnd, IDNO as isize);
        }

        return 0;
    }

    let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog<'_>;
    let Some(dialog) = dialog.as_ref() else {
        return 0;
    };

    match message {
        WM_COMMAND => {
            let id = (wp & 0xffff) as i32;
            let yes = control(hwnd, IDYES as usize);
            if id == IDYES && (lp as HWND == yes || GetFocus() == yes) {
                EndDialog(hwnd, IDYES as isize);
            } else if matches!(id, IDNO | IDCANCEL | IDOK) {
                let approved = id == IDOK && GetFocus() == yes;
                EndDialog(hwnd, (if approved { IDYES } else { IDNO }) as isize);
            } else {
                return 0;
            }
            1
        }
        WM_CLOSE | WM_DISPLAYCHANGE => {
            EndDialog(hwnd, IDNO as isize);
            1
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
        WM_ERASEBKGND => 1,
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => {
            SetTextColor(wp as HDC, TEXT);
            SetBkColor(wp as HDC, MODAL);
            SetDCBrushColor(wp as HDC, MODAL);
            GetStockObject(DC_BRUSH) as isize
        }
        WM_DRAWITEM if lp != 0 => {
            let item = &*(lp as *const DRAWITEMSTRUCT);
            if item.CtlType != ODT_BUTTON {
                return 0;
            }
            if let Ok(mut frame) = dialog.state.control_frame.try_borrow_mut() {
                presentation::control_frame(
                    &mut frame,
                    item,
                    &dialog.state.theme,
                    DASHBOARD,
                    false,
                );
            }
            1
        }
        _ => 0,
    }
}

unsafe fn initialize(hwnd: HWND, dialog: &Dialog<'_>) -> Result<()> {
    let theme = &dialog.state.theme;
    let owner = GetWindow(hwnd, GW_OWNER);
    let area = layout::Layout::read(owner);
    let mut origin = POINT::default();
    ClientToScreen(owner, &mut origin);

    let width = area.px(dialog.prompt.width).min(area.width - 32);
    let padding = area.px(27);
    let label_width = area.px(142);
    let value_width = width - 2 * padding - label_width;
    let row = area.px(27);
    let path_height = path_heights(owner, theme.mono(), value_width, row, dialog.prompt);
    let header_height = area.px(112);
    let notice_height = area.px(67);
    let height = header_height + path_height.iter().sum::<i32>() + notice_height + area.px(92);
    let card = RECT {
        left: (area.width - width) / 2,
        top: (area.height - height) / 2,
        right: (area.width + width) / 2,
        bottom: (area.height + height) / 2,
    };
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
        return Err(os_error("Position confirmation dialog"));
    }

    let screen = GetDC(null_mut());
    if screen.is_null() {
        return Err(os_error("Read confirmation backdrop"));
    }
    let mut frame = dialog.frame.borrow_mut();
    let ready = frame.reserve(screen, area.width, area.height);
    let captured = ready && frame.dim_backdrop(screen, origin, area.width, area.height);
    ReleaseDC(null_mut(), screen);
    if !ready {
        return Err(Error::new(
            ErrorKind::Environment,
            "Allocate confirmation surface",
        ));
    }
    frame.draw(area.width, area.height, |scene| {
        if !captured {
            scene.clear(BACKGROUND);
        }
        presentation::confirmation(
            scene,
            &area,
            theme,
            presentation::ConfirmationCard {
                bounds: card,
                title: dialog.prompt.title,
                question: &dialog.prompt.question,
                labels: [dialog.prompt.paths[0].0, dialog.prompt.paths[1].0],
                rows: path_height,
                notice: dialog.prompt.notice,
            },
        );
    });
    drop(frame);

    let value_x = card.left + padding + label_width;
    let mut value_y = card.top + header_height;
    for ((id, (_, value)), height) in [FIRST_PATH, SECOND_PATH]
        .into_iter()
        .zip(&dialog.prompt.paths)
        .zip(path_height)
    {
        add(
            hwnd,
            id,
            "EDIT",
            value,
            WS_TABSTOP | ES_MULTILINE as u32 | ES_READONLY as u32,
            theme.mono(),
        )?;
        // Native multiline editing keeps the full value selectable and wraps at
        // the field edge. Keyboard scrolling also reaches exceptionally long paths.
        let field = control(hwnd, id);
        SendMessageW(field, EM_SETMARGINS, 3, 0);
        place(hwnd, id, value_x, value_y, value_width, height);
        value_y += height;
    }

    let button_width = area.px(101);
    let button_height = area.px(39);
    let button_top = card.bottom - padding - button_height;
    let mut button_x = card.right - padding - 2 * button_width - area.px(9);
    for (id, label) in [(IDNO, "Não"), (IDYES, "Sim")] {
        add(hwnd, id as usize, "BUTTON", label, WS_TABSTOP, theme.body())?;
        place(
            hwnd,
            id as usize,
            button_x,
            button_top,
            button_width,
            button_height,
        );
        button_x += button_width + area.px(9);
    }

    SetWindowTextW(hwnd, wide(dialog.prompt.title).as_ptr());
    SendMessageW(hwnd, DM_SETDEFID, IDNO as usize, 0);
    SetFocus(control(hwnd, IDNO as usize));
    Ok(())
}

unsafe fn path_heights(
    owner: HWND,
    font: HFONT,
    width: i32,
    row: i32,
    prompt: &Prompt,
) -> [i32; 2] {
    let dc = GetDC(owner);
    if dc.is_null() {
        return [row; 2];
    }

    let previous = SelectObject(dc, font);
    let mut metrics = TEXTMETRICW::default();
    let measured = GetTextMetricsW(dc, &mut metrics) != 0;
    SelectObject(dc, previous);
    ReleaseDC(owner, dc);
    if !measured {
        return [row; 2];
    }

    let capacity = (width / metrics.tmAveCharWidth.max(1)).max(1) as usize;
    prompt.paths.each_ref().map(|(_, value)| {
        let lines = value.encode_utf16().count().div_ceil(capacity).clamp(1, 4);
        row.max(lines as i32 * (metrics.tmHeight + metrics.tmExternalLeading) + 4)
    })
}
