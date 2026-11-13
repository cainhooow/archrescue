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

        Self::new(client.right, client.bottom)
    }

    fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            rail: (width * 247 / 1530).clamp(150, 247),
        }
    }

    pub fn px(&self, reference: i32) -> i32 {
        (reference * self.width.clamp(1100, 1530) + 765) / 1530
    }

    pub fn left(&self) -> i32 {
        self.rail + self.gutter()
    }

    pub fn body_width(&self) -> i32 {
        self.width - self.left() - self.gutter()
    }

    fn gutter(&self) -> i32 {
        if self.width < 960 {
            16
        } else {
            self.px(30)
        }
    }

    pub fn gap(&self) -> i32 {
        self.px(15)
    }

    pub fn split_details(&self) -> bool {
        self.body_width() >= 730
    }

    pub fn detail_columns(&self, section: usize) -> (i32, i32, i32) {
        self.columns(if section == DIAGNOSIS { 60 } else { 42 })
    }

    pub fn columns(&self, leading_percent: i32) -> (i32, i32, i32) {
        let available = self.body_width() - self.gap();
        let left = available * leading_percent / 100;
        let right = self.left() + left + self.gap();

        (left, right, available - left)
    }

    pub fn navigation(&self, index: usize) -> (i32, i32) {
        let divider = if index >= 7 { self.px(24) } else { 0 };
        let top = self.px(103)
            + index as i32 * self.px(if self.height < 690 { 43 } else { 48 })
            + divider;

        (top, self.px(if self.height < 690 { 39 } else { 43 }))
    }

    pub fn utilities(&self) -> i32 {
        self.height - self.px(177)
    }

    pub fn footer(&self) -> i32 {
        self.height - self.px(64)
    }

    pub fn actions(&self) -> i32 {
        self.footer() - self.px(52)
    }

    pub fn body_top(&self, section: usize) -> i32 {
        self.px(174)
            + match section {
                deployment::DEPLOY => 168,
                BACKUP => 55,
                INTEGRITY_PAGE => 39,
                REPORTS => 77,
                VERIFY_PAGE => 130,
                _ => 0,
            }
    }

    pub fn backup_top(&self, app: &App) -> i32 {
        if app.preview.is_some() {
            self.body_top(DASHBOARD) + if app.progress.is_some() { 78 } else { 0 }
        } else {
            self.body_top(BACKUP)
        }
    }

    pub fn backup_form(&self, app: &App) -> i32 {
        let top = self.backup_top(app);
        let height = if app.preview.is_some() {
            230
        } else if app.listing_loaded || app.checkpoints.is_some() {
            270
        } else {
            173
        };

        (top + height + self.gap()).min(self.footer() - 115)
    }

    pub fn body_bottom(&self, section: usize) -> i32 {
        match section {
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
    let (left, right, right_width) = area.columns(60);

    let rail_inset = area.px(17);
    let rail_width = area.rail - 2 * rail_inset;
    for (index, page) in navigation::ORDER.into_iter().enumerate() {
        let (top, height) = area.navigation(index);
        place(hwnd, 100 + page, rail_inset, top, rail_width, height);
    }

    for (index, id) in [TECHNICIAN, REFRESH, CANCEL].into_iter().enumerate() {
        place(
            hwnd,
            id,
            rail_inset,
            area.utilities() + index as i32 * area.px(40),
            rail_width,
            area.px(37),
        );
    }

    let body_end = x + width;
    let footer = area.footer();
    let previous_width = area.px(90);
    let next_width = area.px(104);
    let button_gap = area.px(9);
    let next_x = body_end - next_width;
    let previous_x = next_x - button_gap - previous_width;
    let button_top = footer + area.px(15);
    let button_height = area.px(35);
    place(
        hwnd,
        PREVIOUS,
        previous_x,
        button_top,
        previous_width,
        button_height,
    );
    place(hwnd, NEXT, next_x, button_top, next_width, button_height);
    place(
        hwnd,
        STATUS,
        x + area.px(25),
        footer + area.px(19),
        previous_x - x - area.px(36),
        area.px(34),
    );

    place(hwnd, DISK_DETAILS, x, area.actions(), 168, 30);

    let top = area.body_top(section);
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
            let source_top = area.body_top(DASHBOARD);
            show(hwnd, SOURCE_LABEL, app.preview.is_none());
            show(hwnd, ADDRESS, app.preview.is_none());
            place(hwnd, SOURCE_LABEL, x, source_top, width - 10, 17);
            let field = (width - 16) / 3;
            place(hwnd, CHOOSE_WINDOWS, x, source_top + 20, field, 29);
            place(hwnd, ADDRESS, x + field + 8, source_top + 20, field, 29);
            place(
                hwnd,
                SELECT_RECOMMENDED,
                x + 2 * field + 16,
                source_top + 20,
                field,
                29,
            );
            show(hwnd, CHOOSE_WINDOWS, app.preview.is_none());

            let top = area.backup_top(app);
            let form = area.backup_form(app);
            let bottom = form - area.gap();
            place(hwnd, FILES, x + 11, top + 62, left - 22, bottom - top - 72);
            show(
                hwnd,
                BACKUP_SELECTION,
                app.preview.is_none()
                    && (app.listing_loaded || app.checkpoints.is_some())
                    && app
                        .checkpoints
                        .as_ref()
                        .map_or(!app.entries.is_empty(), |entries| !entries.is_empty()),
            );
            place(
                hwnd,
                BACKUP_SELECTION,
                right + 10,
                top + 42,
                right_width - 20,
                (bottom - top - 54).clamp(1, 102),
            );
            place(hwnd, DESTINATION_LABEL, x + 13, form + 9, width - 26, 17);
            place(hwnd, DEST, x + 19, form + 31, width - 38, 24);
            let mut button_x = x + 13;
            for (id, desired) in [
                (PREVIEW, 106),
                (COPY, 145),
                (CHECKPOINTS, 142),
                (RESUME, 100),
            ] {
                let button_width = desired.min((width - 48) / 4);
                place(hwnd, id, button_x, form + 65, button_width, 32);
                button_x += button_width + 8;
            }
            if app.preview.is_some() {
                place(
                    hwnd,
                    CONTENT,
                    right + 10,
                    top + 99,
                    right_width - 20,
                    (bottom - top - 109).clamp(1, 113),
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
            place(hwnd, DEST, right + 19, top + 67, right_width - 38, 22);
            place(hwnd, EXPORT, right + 13, top + 103, right_width - 26, 31);
        }
        REPAIR_PAGE => {
            place(hwnd, CHOOSE_WINDOWS, x + 13, top + 39, left - 26, 25);
            place(hwnd, ESP_LABEL, x + 13, top + 70, left - 26, 18);
            place(hwnd, ESP, x + 19, top + 91, left - 38, 25);
            place(hwnd, DESTINATION_LABEL, x + 13, top + 121, left - 26, 20);
            place(hwnd, DEST, x + 19, top + 144, left - 38, 25);
            place(hwnd, REPAIR, x + 13, top + 177, left - 26, 31);
            place(
                hwnd,
                CONTENT,
                x + 12,
                top + 222,
                left - 24,
                bottom - top - 232,
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
        VERIFY_PAGE => {
            let card_top = area.body_top(DASHBOARD);
            let has_result = app.session.backup.is_some()
                || app
                    .report
                    .as_ref()
                    .is_some_and(|report| !report.transactions.is_empty());
            show(hwnd, CONTENT, has_result);
            if has_result {
                place(
                    hwnd,
                    CONTENT,
                    x + 13,
                    card_top + 146,
                    left - 26,
                    (bottom - card_top - 159).clamp(1, 206),
                );
            }
            place(
                hwnd,
                CHOOSE_BACKUP,
                right + 13,
                card_top + 61,
                right_width - 26,
                30,
            );
            place(
                hwnd,
                VERIFY,
                right + 13,
                card_top + 105,
                right_width - 26,
                34,
            );
        }
        deployment::DEPLOY | deployment::RESTORE => deployment::layout(hwnd, &area, app),
        console::PAGE => console::layout(hwnd, &area),
        DIAGNOSIS => place(hwnd, CHOOSE_WINDOWS, x + width - 220, top - 31, 220, 26),
        _ => {}
    }

    if pages::structured(section) {
        pages::layout(hwnd, &area, section);
    }
}
