use crate::usecase::TodoUsecase;
use crate::domain::task::{Status, Task};
use crate::usecase::todo::TaskFilter;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

pub mod layout;
pub mod widgets;

#[derive(PartialEq, Eq, Debug)]
pub enum InputMode {
    Normal,
    Add,
    Search,
}

pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

pub struct TuiApp {
    usecase: TodoUsecase,
    tasks: Vec<Task>,
    selected_index: usize,
    should_quit: bool,
    last_refresh: Instant,
    active_tab: Status,
    input_mode: InputMode,
    input_buffer: String,
}

impl TuiApp {
    pub fn new(usecase: TodoUsecase) -> Result<Self> {
        let mut app = Self {
            usecase,
            tasks: Vec::new(),
            selected_index: 0,
            should_quit: false,
            last_refresh: Instant::now(),
            active_tab: Status::Open,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
        };
        app.refresh_tasks()?;
        Ok(app)
    }

    pub fn refresh_tasks(&mut self) -> Result<()> {
        let mut tasks = self.usecase.list_tasks(TaskFilter {
            status: Some(self.active_tab),
            unassigned: false,
        })?;

        // Fuzzy filtering if in Search mode or if query exists
        if !self.input_buffer.is_empty() && (self.input_mode == InputMode::Search || self.input_mode == InputMode::Normal) {
            let matcher = fuzzy_matcher::skim::SkimMatcherV2::default().ignore_case();
            tasks = tasks
                .into_iter()
                .filter(|t| {
                    use fuzzy_matcher::FuzzyMatcher;
                    matcher.fuzzy_match(&t.title, &self.input_buffer).is_some()
                })
                .collect();
        }

        self.tasks = tasks;
        if self.tasks.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.tasks.len() {
            self.selected_index = self.tasks.len() - 1;
        }
        self.last_refresh = Instant::now();
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if self.handle_key_event(key.code)? {
                            terminal.clear()?;
                        }
                    }
                }
            }

            if self.last_refresh.elapsed() > Duration::from_secs(3) {
                self.refresh_tasks()?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, code: KeyCode) -> Result<bool> {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(code),
            InputMode::Add => self.handle_add_key(code),
            InputMode::Search => self.handle_search_key(code),
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('f') | KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.input_buffer = String::new();
                self.refresh_tasks()?;
            }
            KeyCode::Char('a') => {
                self.input_mode = InputMode::Add;
                self.input_buffer = String::new();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection(-1);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.cycle_tab(false)?;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.cycle_tab(true)?;
            }
            KeyCode::Char('s') => {
                self.usecase.sync()?;
                self.refresh_tasks()?;
            }
            KeyCode::Char('m') => {
                return self.handle_edit_task();
            }
            KeyCode::Char('d') => {
                self.handle_done_task()?;
            }
            KeyCode::Char('c') => {
                self.handle_claim_task()?;
            }
            _ => {}
        }
        Ok(false)
    }

    fn move_selection(&mut self, delta: i32) {
        if self.tasks.is_empty() {
            self.selected_index = 0;
            return;
        }
        
        let new_idx = if delta > 0 {
            self.selected_index.saturating_add(delta as usize).min(self.tasks.len() - 1)
        } else {
            self.selected_index.saturating_sub(delta.unsigned_abs() as usize)
        };
        self.selected_index = new_idx;
    }

    fn handle_add_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    self.usecase.add_task(self.input_buffer.clone(), None, None)?;
                }
                self.input_mode = InputMode::Normal;
                self.input_buffer = String::new();
                self.refresh_tasks()?;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer = String::new();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_search_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Enter | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.refresh_tasks()?;
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                self.refresh_tasks()?;
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                self.refresh_tasks()?;
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_edit_task(&mut self) -> Result<bool> {
        let task = match self.tasks.get(self.selected_index) {
            Some(t) => t,
            None => return Ok(false),
        };

        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen)?;

        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path();
        
        let initial_content = format!("{}\n{}", task.title, task.description.as_deref().unwrap_or(""));
        std::fs::write(temp_path, initial_content)?;

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let _ = std::process::Command::new(editor).arg(temp_path).status();

        let content = std::fs::read_to_string(temp_path)?;
        let (title, description) = TodoUsecase::parse_editor_content(&content);
        if !title.is_empty() {
            let mut updated = task.clone();
            updated.title = title;
            updated.description = description;
            updated.updated_at = chrono::Utc::now();
            self.usecase.save_task(&updated)?;
        }

        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        self.refresh_tasks()?;
        Ok(true)
    }

    fn handle_done_task(&mut self) -> Result<()> {
        if let Some(task) = self.tasks.get(self.selected_index) {
            if let Some(id) = task.local_id {
                self.usecase.update_status(id, Status::Close)?;
                self.refresh_tasks()?;
            }
        }
        Ok(())
    }

    fn handle_claim_task(&mut self) -> Result<()> {
        if let Some(task) = self.tasks.get(self.selected_index) {
            if let Some(id) = task.local_id {
                let current_user = std::env::var("USER").unwrap_or_else(|_| "human".to_string());
                self.usecase.claim_task(id, Some(current_user))?;
                self.refresh_tasks()?;
            }
        }
        Ok(())
    }

    fn cycle_tab(&mut self, next: bool) -> Result<()> {
        let tabs = [Status::Open, Status::InProgress, Status::Pending, Status::Close];
        let current_idx = tabs.iter().position(|&s| s == self.active_tab).unwrap_or(0);
        
        let next_idx = if next {
            (current_idx + 1) % tabs.len()
        } else {
            (current_idx + tabs.len() - 1) % tabs.len()
        };
        
        self.active_tab = tabs[next_idx];
        self.selected_index = 0;
        self.refresh_tasks()
    }

    fn render(&self, f: &mut Frame) {
        let (tab_area, list_area, detail_area, file_area, help_area) = layout::get_layout(f.area());

        // [1] Status / Tabs
        widgets::render_tabs(f, tab_area, self.active_tab);

        // [2] Task List
        widgets::render_task_list(f, list_area, &self.tasks, self.selected_index);

        // [3] Task Detail (Markdown)
        let (detail_text, detail_title) = if let Some(task) = self.tasks.get(self.selected_index) {
            let text = task.description.as_deref().unwrap_or("No description").to_string();
            let title = format!(" Details: #{} {} ", task.local_id.unwrap_or(0), task.title);
            (text, title)
        } else {
            ("No task selected".to_string(), " Details ".to_string())
        };
        widgets::render_markdown(f, detail_area, &detail_text, &detail_title);

        // [4] Related Files
        let files = self.tasks.get(self.selected_index)
            .map(|t| t.linked_files.clone())
            .unwrap_or_default();
        widgets::render_related_files(f, file_area, &files);

        // [5] Key Help / Search
        widgets::render_help_bar(f, help_area, &self.input_mode, &self.input_buffer);

        // Popup for Add Mode
        if self.input_mode == InputMode::Add {
            widgets::render_add_popup(f, &self.input_buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_app() -> (TuiApp, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone()).unwrap();
        let usecase = TodoUsecase::new(root).unwrap();
        
        // Add some dummy tasks
        usecase.add_task("Apple".to_string(), None, None).unwrap();
        usecase.add_task("Banana".to_string(), None, None).unwrap();
        
        let app = TuiApp::new(usecase).unwrap();
        (app, dir)
    }

    #[test]
    fn test_tab_cycling() {
        let (mut app, _dir) = setup_app();
        assert_eq!(app.active_tab, Status::Open);
        
        app.cycle_tab(true).unwrap();
        assert_eq!(app.active_tab, Status::InProgress);
        
        app.cycle_tab(true).unwrap();
        assert_eq!(app.active_tab, Status::Pending);
        
        app.cycle_tab(true).unwrap();
        assert_eq!(app.active_tab, Status::Close);
        
        app.cycle_tab(true).unwrap();
        assert_eq!(app.active_tab, Status::Open);
        
        app.cycle_tab(false).unwrap();
        assert_eq!(app.active_tab, Status::Close);
    }

    #[test]
    fn test_navigation() {
        let (mut app, _dir) = setup_app();
        assert_eq!(app.selected_index, 0);
        
        app.handle_key_event(KeyCode::Char('j')).unwrap();
        assert_eq!(app.selected_index, 1);
        
        // Boundaries
        app.handle_key_event(KeyCode::Char('j')).unwrap();
        assert_eq!(app.selected_index, 1);
        
        app.handle_key_event(KeyCode::Char('k')).unwrap();
        assert_eq!(app.selected_index, 0);
        
        app.handle_key_event(KeyCode::Char('k')).unwrap();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_empty_list_navigation() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone()).unwrap();
        let usecase = TodoUsecase::new(root).unwrap();
        let mut app = TuiApp::new(usecase).unwrap(); // No tasks
        
        assert_eq!(app.tasks.len(), 0);
        assert_eq!(app.selected_index, 0);
        
        // Should not panic
        app.handle_key_event(KeyCode::Char('j')).unwrap();
        assert_eq!(app.selected_index, 0);
        app.handle_key_event(KeyCode::Char('k')).unwrap();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_done_action_logic() {
        let (mut app, _dir) = setup_app(); // 2 tasks in Open
        assert_eq!(app.tasks.len(), 2);
        
        // Mark first task as Done
        app.handle_key_event(KeyCode::Char('d')).unwrap();
        
        // Should have 1 task left in Open tab
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].title, "Banana");
        
        // Switch to Done tab (Open -> InProgress -> Pending -> Close)
        app.cycle_tab(true).unwrap(); // InProgress
        app.cycle_tab(true).unwrap(); // Pending
        app.cycle_tab(true).unwrap(); // Close
        
        assert_eq!(app.active_tab, Status::Close);
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].title, "Apple");
    }

    #[test]
    fn test_claim_action_logic() {
        let (mut app, _dir) = setup_app();
        app.handle_key_event(KeyCode::Char('c')).unwrap();
        
        // Apple is now InProgress, so it disappears from Open tab
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].title, "Banana");
        
        // Switch to InProgress tab
        app.cycle_tab(true).unwrap();
        assert_eq!(app.active_tab, Status::InProgress);
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].title, "Apple");
        assert!(app.tasks[0].assignee.is_some());
    }

    #[test]
    fn test_selection_index_safety_on_filter() {
        let (mut app, _dir) = setup_app();
        app.selected_index = 1; // Select Banana
        
        // Start search for "Apple"
        app.handle_key_event(KeyCode::Char('f')).unwrap();
        app.input_buffer = "Apple".to_string();
        app.refresh_tasks().unwrap();
        
        assert_eq!(app.tasks.len(), 1);
        assert_eq!(app.tasks[0].title, "Apple");
        // Index should have been clamped to 0
        assert_eq!(app.selected_index, 0);
        
        // Clear search and buffer
        app.handle_key_event(KeyCode::Esc).unwrap();
        app.input_buffer = String::new();
        app.refresh_tasks().unwrap();
        
        assert_eq!(app.tasks.len(), 2);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_search_backspace_edge_cases() {
        let (mut app, _dir) = setup_app();
        
        // Enter search mode
        app.handle_key_event(KeyCode::Char('f')).unwrap();
        assert_eq!(app.input_buffer, "".to_string());
        
        // Backspace on empty query should not panic
        app.handle_key_event(KeyCode::Backspace).unwrap();
        assert_eq!(app.input_buffer, "".to_string());
        
        // Type and delete
        app.handle_key_event(KeyCode::Char('x')).unwrap();
        assert_eq!(app.input_buffer, "x".to_string());
        app.handle_key_event(KeyCode::Backspace).unwrap();
        assert_eq!(app.input_buffer, "".to_string());
    }

    #[test]
    fn test_render_buffer() {
        let (app, _dir) = setup_app();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|f| app.render(f)).unwrap();
        
        let buffer = terminal.backend().buffer();
        let content = format!("{:?}", buffer);
        
        // Check for key UI elements
        assert!(content.contains("Status"));
        assert!(content.contains("Tasks"));
        assert!(content.contains("Details"));
        assert!(content.contains("Files"));
        assert!(content.contains("Help"));
        
        // Check for specific task title
        assert!(content.contains("Apple"));
    }

    #[test]
    fn test_add_mode_popup_rendering() {
        let (mut app, _dir) = setup_app();
        app.handle_key_event(KeyCode::Char('a')).unwrap();
        assert_eq!(app.input_mode, InputMode::Add);
        
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| app.render(f)).unwrap();
        
        let buffer = terminal.backend().buffer();
        let content = format!("{:?}", buffer);
        
        assert!(content.contains("Create New Task"));
    }

    #[test]
    fn test_detail_ghosting_prevention() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        TodoUsecase::init(root.clone()).unwrap();
        let usecase = TodoUsecase::new(root).unwrap();
        
        // Long title vs Short title
        usecase.add_task("Very Long Task Title Indeed".to_string(), None, None).unwrap();
        usecase.add_task("Short".to_string(), None, None).unwrap();
        
        let mut app = TuiApp::new(usecase).unwrap();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        // 1. Draw long task
        terminal.draw(|f| app.render(f)).unwrap();
        assert!(format!("{:?}", terminal.backend().buffer()).contains("Very Long"));
        
        // 2. Move to short task
        app.handle_key_event(KeyCode::Char('j')).unwrap();
        terminal.draw(|f| app.render(f)).unwrap();
        
        let buffer_str = format!("{:?}", terminal.backend().buffer());
        assert!(buffer_str.contains("Short"));
        // "Indeed" should NOT be in the buffer anymore
        assert!(!buffer_str.contains("Indeed"), "Ghosting detected! 'Indeed' should have been cleared.");
    }
}
