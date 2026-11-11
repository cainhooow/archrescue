//! Client-space geometry. Page bodies scroll independently of the actions.

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
            rail: if client.right < 960 { 146 } else { 218 },
        }
    }

    pub fn left(&self) -> i32 {
        self.rail + 22
    }

    pub fn body_width(&self) -> i32 {
        self.width - self.left() - 22
    }

    pub fn actions(&self) -> i32 {
        self.height - 100
    }

    pub fn destination(&self) -> i32 {
        self.actions() - 37
    }

    pub fn body_top(section: usize) -> i32 {
        match section {
            deployment::DEPLOY => 226,
            BACKUP => 173,
            deployment::RESTORE | REPAIR_PAGE | INTEGRITY_PAGE => 145,
            _ => 108,
        }
    }

    pub fn body_bottom(&self, section: usize) -> i32 {
        match section {
            DASHBOARD | DIAGNOSIS | DISKS | VERIFY_PAGE | INTEGRITY_PAGE => self.actions() - 8,
            _ => self.destination() - 8,
        }
    }
}

pub unsafe fn arrange(hwnd: HWND, section: usize) {
    let area = Layout::read(hwnd);
    let x = area.left();
    let width = area.body_width();

    for index in 0..i18n::SECTIONS.len() {
        place(
            hwnd,
            100 + index,
            9,
            76 + index as i32 * 37,
            area.rail - 18,
            34,
        );
    }
    place(hwnd, REFRESH, 9, area.height - 78, area.rail - 18, 30);
    place(hwnd, CANCEL, 9, area.height - 40, area.rail - 18, 30);
    place(hwnd, TECHNICIAN, area.width - 176, 22, 154, 34);
    place(hwnd, CONSOLE, x + 228, area.actions(), 204, 32);

    place(hwnd, SOURCE_LABEL, x, 113, 62, 22);
    place(hwnd, ADDRESS, x + 62, 108, width - 339, 30);
    place(hwnd, OPEN, x + width - 269, 108, 70, 30);
    place(hwnd, UP, x + width - 191, 108, 70, 30);
    place(hwnd, BACKUP_RESULTS, x + width - 113, 108, 113, 30);
    if section == BACKUP {
        place(hwnd, pages::PAGE_HINT, x, 143, width, 22);
    }
    place(hwnd, ESP_LABEL, x, 113, 148, 22);
    place(hwnd, ESP, x + 148, 108, width - 148, 30);
    place(hwnd, DESTINATION_LABEL, x, area.destination() + 5, 62, 22);
    place(hwnd, DEST, x + 62, area.destination(), width - 62, 30);

    let top = Layout::body_top(section);
    let bottom = area.body_bottom(section);
    place(hwnd, FILES, x, top, width, bottom - top);
    place(
        hwnd,
        CONTENT,
        x + 12,
        top + 10,
        width - 24,
        bottom - top - 20,
    );

    let button_width = (width - 24) / 4;
    for (index, id) in [PREVIEW, COPY, CHECKPOINTS, RESUME].into_iter().enumerate() {
        place(
            hwnd,
            id,
            x + index as i32 * (button_width + 8),
            area.actions(),
            button_width,
            32,
        );
    }
    for id in [EXPORT, VERIFY, REPAIR, NEXT] {
        place(hwnd, id, x, area.actions(), 220, 32);
    }

    place(hwnd, STATUS, x + 12, area.height - 51, width - 24, 42);
    deployment::layout(hwnd, &area, section);
    if pages::structured(section) {
        pages::layout(hwnd, &area, section);
    }
}
