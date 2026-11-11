//! Client-space geometry. Scrollable native regions stay above the fixed footer.

use super::*;

pub struct Layout {
    pub width: i32,
    pub height: i32,
    pub rail: i32,
}

impl Layout {
    pub unsafe fn read(hwnd: HWND) -> Self {
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client);

        Self {
            width: client.right,
            height: client.bottom,
            rail: if client.right < 960 { 150 } else { 218 },
        }
    }

    pub fn left(&self) -> i32 {
        self.rail + 22
    }

    pub fn body_width(&self) -> i32 {
        self.width - self.left() - 22
    }

    pub fn left_column(&self) -> i32 {
        (self.body_width() - 12) * 60 / 100
    }

    pub fn footer(&self) -> i32 {
        self.height - 51
    }

    pub fn actions(&self) -> i32 {
        self.footer() - 44
    }

    pub fn destination(&self) -> i32 {
        self.actions() - 36
    }

    pub fn body_top(section: usize) -> i32 {
        match section {
            deployment::DEPLOY => 295,
            BACKUP => 169,
            INTEGRITY_PAGE => 166,
            REPORTS => 204,
            VERIFY_PAGE => 257,
            _ => 127,
        }
    }

    pub fn body_bottom(&self, section: usize) -> i32 {
        match section {
            BACKUP => self.destination() - 18,
            VERIFY_PAGE | INTEGRITY_PAGE => self.actions() - 8,
            _ => self.footer() - 12,
        }
    }
}

pub unsafe fn arrange(hwnd: HWND, app: &App) {
    let area = Layout::read(hwnd);
    let section = app.section;
    let x = area.left();
    let width = area.body_width();
    let left = area.left_column();
    let right = x + left + 12;
    let right_width = width - left - 12;

    for (index, page) in navigation::ORDER.into_iter().enumerate() {
        let group_gap = if index >= 7 { 11 } else { 0 };
        place(
            hwnd,
            100 + page,
            10,
            76 + index as i32 * 34 + group_gap,
            area.rail - 20,
            31,
        );
    }

    for (index, id) in [TECHNICIAN, REFRESH, CONSOLE, CANCEL]
        .into_iter()
        .enumerate()
    {
        place(
            hwnd,
            id,
            10,
            area.height - 125 + index as i32 * 29,
            area.rail - 20,
            26,
        );
    }

    place(hwnd, PREVIOUS, area.width - 194, area.footer() + 10, 80, 30);
    place(hwnd, NEXT, area.width - 106, area.footer() + 10, 84, 30);
    place(hwnd, STATUS, x + 20, area.footer() + 8, width - 204, 34);

    place(hwnd, SOURCE_LABEL, x, 135, 58, 22);
    place(hwnd, ADDRESS, x + 59, 130, width - 215, 29);
    place(hwnd, OPEN, x + width - 148, 130, 70, 29);
    place(hwnd, UP, x + width - 70, 130, 70, 29);
    place(hwnd, DISK_DETAILS, x, area.actions(), 168, 30);

    let top = Layout::body_top(section);
    let bottom = area.body_bottom(section);
    place(
        hwnd,
        CONTENT,
        x + 13,
        top + 41,
        width - 26,
        bottom - top - 49,
    );
    place(hwnd, FILES, x + 11, top + 62, width - 22, bottom - top - 72);

    match section {
        BACKUP => {
            place(
                hwnd,
                DESTINATION_LABEL,
                x + 13,
                area.destination() + 5,
                62,
                22,
            );
            place(hwnd, DEST, x + 76, area.destination(), width - 89, 28);
            let button_width = (width - 50) / 4;
            for (index, id) in [PREVIEW, COPY, CHECKPOINTS, RESUME].into_iter().enumerate() {
                place(
                    hwnd,
                    id,
                    x + 13 + index as i32 * (button_width + 8),
                    area.actions(),
                    button_width,
                    30,
                );
            }
            if app.preview.is_some() {
                place(
                    hwnd,
                    CONTENT,
                    right + 10,
                    top + 99,
                    right_width - 20,
                    bottom - top - 109,
                );
            }
        }
        REPORTS => {
            place(
                hwnd,
                CONTENT,
                x + 12,
                top + 39,
                left - 24,
                bottom - top - 50,
            );
            place(
                hwnd,
                DESTINATION_LABEL,
                right + 13,
                top + 42,
                right_width - 26,
                20,
            );
            place(hwnd, DEST, right + 13, top + 64, right_width - 26, 28);
            place(hwnd, EXPORT, right + 13, top + 103, right_width - 26, 31);
        }
        REPAIR_PAGE => {
            place(hwnd, ESP_LABEL, x + 13, top + 42, left - 26, 20);
            place(hwnd, ESP, x + 13, top + 64, left - 26, 28);
            place(hwnd, DESTINATION_LABEL, x + 13, top + 98, left - 26, 20);
            place(hwnd, DEST, x + 13, top + 120, left - 26, 28);
            place(hwnd, REPAIR, x + 13, top + 158, left - 26, 31);
            place(
                hwnd,
                CONTENT,
                x + 12,
                top + 236,
                left - 24,
                bottom - top - 247,
            );
            place(
                hwnd,
                REPAIR_UNAVAILABLE,
                right + 13,
                top + 121,
                right_width - 26,
                30,
            );
        }
        VERIFY_PAGE => place(hwnd, VERIFY, x + width - 184, area.actions(), 184, 30),
        deployment::DEPLOY | deployment::RESTORE => deployment::layout(hwnd, &area, section),
        _ => {}
    }

    if pages::structured(section) {
        pages::layout(hwnd, &area, section);
    }
}
