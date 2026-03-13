use ratatui::prelude::*;
use termimad::{MadSkin};
use crate::domain::task::{Status, Task};
use super::InputMode;
use std::time::Instant;

pub fn render_tabs(f: &mut Frame, area: Rect, active_tab: Status) {
    let titles = vec![" [1] Open ", " [2] Doing ", " [3] Pending ", " [4] Done "];
    let current_idx = match active_tab {
        Status::Open => 0,
        Status::InProgress => 1,
        Status::Pending => 2,
        Status::Close => 3,
    };
    
    let tabs = ratatui::widgets::Tabs::new(titles)
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Status "))
        .select(current_idx)
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, area);
}

pub fn render_task_list(f: &mut Frame, area: Rect, tasks: &[Task], selected_index: usize) {
    let list_items: Vec<ratatui::widgets::ListItem> = tasks.iter().enumerate().map(|(i, t)| {
        let style = if i == selected_index {
            Style::default().bg(Color::DarkGray).fg(Color::Cyan)
        } else {
            Style::default()
        };
        let status_mark = match t.status {
            Status::Close => "[x]",
            Status::InProgress => "[-]",
            _ => "[ ]",
        };
        let assignee_mark = if t.assignee.is_some() { " *" } else { "" };
        ratatui::widgets::ListItem::new(format!("{}{} #{} {}", status_mark, assignee_mark, t.local_id.unwrap_or(0), t.title)).style(style)
    }).collect();

    let list = ratatui::widgets::List::new(list_items)
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Tasks "));
    f.render_widget(list, area);
}

pub fn render_file_selection(f: &mut Frame, area: Rect, files: &[String], selected_index: usize, linked_files: &[String]) {
    let list_items: Vec<ratatui::widgets::ListItem> = files.iter().enumerate().map(|(i, path)| {
        let is_linked = linked_files.contains(path);
        let style = if i == selected_index {
            Style::default().bg(Color::DarkGray).fg(Color::Yellow)
        } else {
            Style::default()
        };
        let mark = if is_linked { "[x]" } else { "[ ]" };
        ratatui::widgets::ListItem::new(format!("{} {}", mark, path)).style(style)
    }).collect();

    let list = ratatui::widgets::List::new(list_items)
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Select Files (Space to Toggle, A/Esc to Exit) "));
    f.render_widget(list, area);
}

pub fn render_markdown(f: &mut Frame, area: Rect, text: &str, title: &str) {
    let block = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .title(title);
    f.render_widget(block, area);

    let inner_area = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    f.render_widget(ratatui::widgets::Clear, inner_area);

    if inner_area.width == 0 || inner_area.height == 0 {
        return;
    }

    let skin = MadSkin::default();
    let fmt_text = skin.text(text, Some(inner_area.width as usize));
    
    let paragraph = ratatui::widgets::Paragraph::new(fmt_text.to_string())
        .wrap(ratatui::widgets::Wrap { trim: false });
    
    f.render_widget(paragraph, inner_area);
}

pub fn render_related_files(f: &mut Frame, area: Rect, files: &[String]) {
    let files_text = files.iter().map(|f| format!("- {}", f)).collect::<Vec<_>>().join("\n");
    f.render_widget(ratatui::widgets::Paragraph::new(files_text)
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Files ")), area);
}

pub fn render_help_bar(f: &mut Frame, area: Rect, input_mode: &InputMode, input_buffer: &str, info_message: &Option<(String, Instant)>) {
    if let Some((msg, _)) = info_message {
        let style = if msg.starts_with("Error") {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        f.render_widget(ratatui::widgets::Paragraph::new(msg.as_str())
            .style(style)
            .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Notification ")), area);
        return;
    }

    if *input_mode == InputMode::Search {
        f.render_widget(ratatui::widgets::Paragraph::new(format!(" SEARCH: {}█", input_buffer))
            .style(Style::default().fg(Color::Yellow))
            .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Search (Esc to close) ")), area);
    } else {
        let help_text = if input_buffer.is_empty() {
            " q:Quit | s:Sync | h/l:Tabs | j/k:Move | a:Add | A:Attach | m:Edit | d:Done | /:Find "
        } else {
            " (FILTER ACTIVE) /:Clear Filter | q:Quit | s:Sync | h/l:Tabs | j/k:Move | a:Add | A:Attach | m:Edit | d:Done "
        };
        f.render_widget(ratatui::widgets::Paragraph::new(help_text)
            .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Help ")), area);
    }
}

pub fn render_add_popup(f: &mut Frame, input_buffer: &str) {
    let area = super::layout::centered_rect(60, 20, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(
        ratatui::widgets::Paragraph::new(input_buffer)
            .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Create New Task (Enter to save, Esc to cancel) ")),
        area,
    );
}

pub fn render_attach_popup(f: &mut Frame, input_buffer: &str) {
    let area = super::layout::centered_rect(60, 20, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(
        ratatui::widgets::Paragraph::new(input_buffer)
            .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title(" Attach Files (Comma separated, Enter to save, Esc to cancel) ")),
        area,
    );
}
