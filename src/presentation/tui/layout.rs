use ratatui::prelude::*;

pub fn get_layout(area: Rect) -> (Rect, Rect, Rect, Rect, Rect) {
    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // [1] Tabs
            Constraint::Min(1),    // [2,3,4] Main body
            Constraint::Length(3), // [5] Status Line
        ])
        .split(area);

    let main_horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // [2] Task List
            Constraint::Percentage(60), // [3,4] Details
        ])
        .split(vertical_chunks[1]);

    let detail_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(70), // [3] Task Detail
            Constraint::Percentage(30), // [4] Related Files
        ])
        .split(main_horizontal[1]);

    (
        vertical_chunks[0],  // Tabs
        main_horizontal[0],  // List
        detail_vertical[0],  // Detail
        detail_vertical[1],  // Related Files
        vertical_chunks[2],  // Status Line
    )
}

/// helper function to create a centered rect using up to certain % of the available rect `r`
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
