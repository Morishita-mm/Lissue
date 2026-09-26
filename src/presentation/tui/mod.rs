use crate::domain::task::{Status, Task};
use crate::usecase::TodoUsecase;
use crate::usecase::todo::TaskFilter;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::{self, Stdout};
use std::time::{Duration, Instant};

pub mod layout;
pub mod widgets;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum InputMode {
    Normal,
    Add,
    Attach,
    FileSelect,
    Search,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum ViewMode {
    Tasks,
    Files,
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

/// Owns the terminal state while an external editor is running.
///
/// The TUI leaves raw mode and the alternate screen before invoking an editor.  This
/// guard makes restoration happen even when the editor cannot be started, exits with a
/// failure, or the edited file cannot be read afterwards.
struct TerminalSuspension {
    restored: bool,
}

impl TerminalSuspension {
    fn new() -> Result<Self> {
        let suspension = Self { restored: false };
        disable_raw_mode()?;

        if let Err(error) = execute!(io::stdout(), LeaveAlternateScreen) {
            // Raw mode was already disabled, so repair it before returning the error.
            let _ = enable_raw_mode();
            return Err(error.into());
        }

        Ok(suspension)
    }

    fn restore(&mut self) -> Result<()> {
        if self.restored {
            return Ok(());
        }

        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        self.restored = true;
        Ok(())
    }
}

impl Drop for TerminalSuspension {
    fn drop(&mut self) {
        if !self.restored {
            let _ = enable_raw_mode();
            let _ = execute!(io::stdout(), EnterAlternateScreen);
        }
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
    view_mode: ViewMode,
    input_buffer: String,
    project_files: Vec<String>,
    filtered_project_files: Vec<String>,
    file_selected_index: usize,
    info_message: Option<(String, Instant)>,
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
            view_mode: ViewMode::Tasks,
            input_buffer: String::new(),
            project_files: Vec::new(),
            filtered_project_files: Vec::new(),
            file_selected_index: 0,
            info_message: None,
        };
        app.refresh_tasks()?;
        Ok(app)
    }

    pub fn set_info(&mut self, message: &str) {
        self.info_message = Some((message.to_string(), Instant::now()));
    }

    pub fn refresh_tasks(&mut self) -> Result<()> {
        let mut tasks = self.usecase.list_tasks(TaskFilter {
            status: Some(self.active_tab),
            unassigned: false,
        })?;

        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default().ignore_case();

        // 1. Filter tasks
        if !self.input_buffer.is_empty() && self.view_mode == ViewMode::Tasks {
            use fuzzy_matcher::FuzzyMatcher;
            tasks.retain(|t| matcher.fuzzy_match(&t.title, &self.input_buffer).is_some());
        }
        self.tasks = tasks;

        // 2. Filter project files
        let mut filtered_files = self.project_files.clone();
        if !self.input_buffer.is_empty() && self.view_mode == ViewMode::Files {
            use fuzzy_matcher::FuzzyMatcher;
            filtered_files.retain(|f| matcher.fuzzy_match(f, &self.input_buffer).is_some());
        }
        self.filtered_project_files = filtered_files;

        // Index clamping for tasks
        if self.tasks.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.tasks.len() {
            self.selected_index = self.tasks.len() - 1;
        }

        // Index clamping for files
        if self.filtered_project_files.is_empty() {
            self.file_selected_index = 0;
        } else if self.file_selected_index >= self.filtered_project_files.len() {
            self.file_selected_index = self.filtered_project_files.len() - 1;
        }

        self.last_refresh = Instant::now();
        Ok(())
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|f| self.render(f))?;

            if let Some((_, time)) = self.info_message
                && time.elapsed() > Duration::from_secs(3)
            {
                self.info_message = None;
            }

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                let key_result = self.handle_key_event(key);
                self.handle_key_result(terminal, key_result)?;
            }

            if self.last_refresh.elapsed() > Duration::from_secs(3) {
                let _ = self.refresh_tasks();
            }
        }
        Ok(())
    }

    fn handle_key_event<K>(&mut self, key: K) -> Result<bool>
    where
        K: Into<KeyEvent>,
    {
        let key = key.into();

        // A modified Ctrl-C is not the plain `c` claim shortcut.  Ignore it in
        // every mode so it can never claim a task or be inserted into an input.
        let is_ctrl_c = matches!(key.code, KeyCode::Char('\u{3}'))
            || (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(
                    key.code,
                    KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('\u{3}')
                ));
        if is_ctrl_c {
            return Ok(false);
        }

        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }

        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key.code),
            InputMode::Add => self.handle_add_key(key.code),
            InputMode::Attach => self.handle_attach_key(key.code),
            InputMode::FileSelect => self.handle_file_select_key(key.code),
            InputMode::Search => self.handle_search_key(key.code),
        }
    }

    fn handle_key_result<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        result: Result<bool>,
    ) -> Result<()> {
        match result {
            Ok(true) => terminal.clear()?,
            Ok(false) => {}
            Err(error) => {
                // Re-entering the alternate screen invalidates Ratatui's frame
                // cache. Clear it on failures too, not only after successful edits.
                terminal.clear()?;
                self.set_info(&format!("Error: {}", error));
            }
        }
        Ok(())
    }

    fn handle_normal_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('/') | KeyCode::Char('?') => {
                self.input_mode = InputMode::Search;
                self.input_buffer = String::new();
                self.refresh_tasks()?;
            }
            KeyCode::Char('a') => {
                self.input_mode = InputMode::Add;
                self.input_buffer = String::new();
            }
            KeyCode::Char('A') => {
                if !self.tasks.is_empty() {
                    self.project_files = self.usecase.list_project_files()?;
                    self.view_mode = ViewMode::Files;
                    self.input_mode = InputMode::FileSelect;
                    self.input_buffer = String::new();
                    self.refresh_tasks()?;
                }
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
            self.selected_index
                .saturating_add(delta as usize)
                .min(self.tasks.len() - 1)
        } else {
            self.selected_index
                .saturating_sub(delta.unsigned_abs() as usize)
        };
        self.selected_index = new_idx;
    }

    fn handle_add_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Enter => {
                if !self.input_buffer.is_empty() {
                    self.usecase
                        .add_task(self.input_buffer.clone(), None, None)?;
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

    fn handle_attach_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Enter => {
                if !self.input_buffer.is_empty()
                    && let Some(task) = self.tasks.get(self.selected_index)
                    && let Some(id) = task.local_id
                {
                    let paths: Vec<String> = self
                        .input_buffer
                        .split([',', ' '])
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    self.usecase.attach_files(id, paths)?;
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

    fn handle_file_select_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Esc | KeyCode::Char('A') | KeyCode::Char('q') => {
                self.view_mode = ViewMode::Tasks;
                self.input_mode = InputMode::Normal;
                self.input_buffer = String::new();
                self.refresh_tasks()?;
            }
            KeyCode::Char('/') | KeyCode::Char('?') => {
                self.input_mode = InputMode::Search;
                self.input_buffer = String::new();
                self.refresh_tasks()?;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.filtered_project_files.is_empty()
                    && self.file_selected_index < self.filtered_project_files.len() - 1
                {
                    self.file_selected_index += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.file_selected_index > 0 {
                    self.file_selected_index -= 1;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let (Some(task), Some(file_path)) = (
                    self.tasks.get(self.selected_index),
                    self.filtered_project_files.get(self.file_selected_index),
                ) && let Some(id) = task.local_id
                {
                    if task.linked_files.contains(file_path) {
                        self.usecase.detach_file(id, file_path)?;
                    } else {
                        self.usecase.attach_files(id, vec![file_path.clone()])?;
                    }
                    self.refresh_tasks()?;
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_search_key(&mut self, code: KeyCode) -> Result<bool> {
        match code {
            KeyCode::Enter | KeyCode::Esc => {
                self.input_mode = if self.view_mode == ViewMode::Files {
                    InputMode::FileSelect
                } else {
                    InputMode::Normal
                };
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

        // Prepare the file before changing terminal state.  If this fails, the TUI
        // remains untouched and no task can be written.
        let temp_file = tempfile::NamedTempFile::new()?;
        let temp_path = temp_file.path();
        let initial_content = format!(
            "{}\n{}",
            task.title,
            task.description.as_deref().unwrap_or("")
        );
        std::fs::write(temp_path, initial_content)?;

        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
        let editor_status = {
            let mut suspension = TerminalSuspension::new()?;
            let command_result = std::process::Command::new(editor).arg(temp_path).status();
            let restore_result = suspension.restore();

            match (command_result, restore_result) {
                (Ok(status), Ok(())) => Ok(status),
                (Err(command_error), Ok(())) => Err(command_error.into()),
                (Ok(_), Err(restore_error)) => Err(restore_error),
                (Err(command_error), Err(restore_error)) => Err(anyhow::anyhow!(
                    "Editor failed to run: {}; terminal restore failed: {}",
                    command_error,
                    restore_error
                )),
            }
        }?;

        if !editor_status.success() {
            return Err(anyhow::anyhow!(
                "Editor failed to exit successfully: {}",
                editor_status
            ));
        }

        // Do not parse or save until both the editor and terminal restoration have
        // succeeded.  Read failures therefore leave the original task unchanged.
        let content = std::fs::read_to_string(temp_path)?;
        let (title, description) = TodoUsecase::parse_editor_content(&content);
        if !title.is_empty() {
            let mut updated = task.clone();
            updated.title = title;
            updated.description = description;
            updated.updated_at = chrono::Utc::now();
            if let Err(save_error) = self.usecase.save_task(&updated) {
                // `save_task` persists the database before the JSON mirror.  Restore
                // the original snapshot if the second write fails so an editor/save
                // error cannot leave the task changed in the database.
                return match self.usecase.save_task(task) {
                    Ok(()) => Err(save_error),
                    Err(rollback_error) => Err(anyhow::anyhow!(
                        "Failed to save edited task: {}; rollback failed: {}",
                        save_error,
                        rollback_error
                    )),
                };
            }
        }

        self.refresh_tasks()?;
        Ok(true)
    }

    fn handle_done_task(&mut self) -> Result<()> {
        if let Some(task) = self.tasks.get(self.selected_index)
            && let Some(id) = task.local_id
        {
            self.usecase.update_status(id, Status::Close)?;
            self.refresh_tasks()?;
        }
        Ok(())
    }

    fn handle_claim_task(&mut self) -> Result<()> {
        if let Some(task) = self.tasks.get(self.selected_index)
            && let Some(id) = task.local_id
        {
            let current_user = std::env::var("USER").unwrap_or_else(|_| "human".to_string());
            self.usecase.claim_task(id, Some(current_user))?;
            self.refresh_tasks()?;
        }
        Ok(())
    }

    fn cycle_tab(&mut self, next: bool) -> Result<()> {
        let tabs = [
            Status::Open,
            Status::InProgress,
            Status::Pending,
            Status::Close,
        ];
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

        // [2] Task List or File Selection
        if self.view_mode == ViewMode::Files {
            let linked_files = self
                .tasks
                .get(self.selected_index)
                .map(|t| t.linked_files.as_slice())
                .unwrap_or(&[]);
            widgets::render_file_selection(
                f,
                list_area,
                &self.filtered_project_files,
                self.file_selected_index,
                linked_files,
            );
        } else {
            widgets::render_task_list(f, list_area, &self.tasks, self.selected_index);
        }

        // [3] Task Detail (Markdown)
        let (detail_text, detail_title) = if let Some(task) = self.tasks.get(self.selected_index) {
            let text = task
                .description
                .as_deref()
                .unwrap_or("No description")
                .to_string();
            let title = format!(" Details: #{} {} ", task.local_id.unwrap_or(0), task.title);
            (text, title)
        } else {
            ("No task selected".to_string(), " Details ".to_string())
        };
        widgets::render_markdown(f, detail_area, &detail_text, &detail_title);

        // [4] Related Files
        let files = self
            .tasks
            .get(self.selected_index)
            .map(|t| t.linked_files.clone())
            .unwrap_or_default();
        widgets::render_related_files(f, file_area, &files);

        // [5] Key Help / Search / Notification
        widgets::render_help_bar(
            f,
            help_area,
            &self.input_mode,
            &self.input_buffer,
            &self.info_message,
        );

        // Popups
        match self.input_mode {
            InputMode::Add => widgets::render_add_popup(f, &self.input_buffer),
            InputMode::Attach => widgets::render_attach_popup(f, &self.input_buffer),
            _ => {}
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
    fn test_failed_key_result_clears_frame_cache() {
        let (mut app, _dir) = setup_app();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Two draws make Ratatui's previous and current buffers identical.
        terminal.draw(|f| app.render(f)).unwrap();
        terminal.draw(|f| app.render(f)).unwrap();
        // Simulate the alternate screen being cleared by an external editor.
        terminal.backend_mut().clear().unwrap();

        app.handle_key_result(
            &mut terminal,
            Err(anyhow::anyhow!("editor failed to start")),
        )
        .unwrap();
        terminal.draw(|f| app.render(f)).unwrap();

        let content = format!("{:?}", terminal.backend().buffer());
        assert!(content.contains("Apple"));
    }

    #[test]
    fn test_ctrl_c_does_not_claim_in_normal_mode() {
        let (mut app, _dir) = setup_app();
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

        app.handle_key_event(ctrl_c).unwrap();

        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.tasks.len(), 2);
        assert!(
            app.tasks
                .iter()
                .all(|task| { task.status == Status::Open && task.assignee.is_none() })
        );
    }

    #[test]
    fn test_ctrl_c_is_ignored_in_every_input_mode() {
        for input_mode in [
            InputMode::Normal,
            InputMode::Add,
            InputMode::Attach,
            InputMode::FileSelect,
            InputMode::Search,
        ] {
            let (mut app, _dir) = setup_app();
            app.input_mode = input_mode;
            app.input_buffer = "before".to_string();
            let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

            app.handle_key_event(ctrl_c).unwrap();

            assert_eq!(app.input_mode, input_mode);
            assert_eq!(app.input_buffer, "before");
            assert_eq!(app.tasks.len(), 2);
            assert!(
                app.tasks
                    .iter()
                    .all(|task| { task.status == Status::Open && task.assignee.is_none() })
            );
        }
    }

    #[test]
    fn test_non_press_ctrl_c_is_ignored() {
        let (mut app, _dir) = setup_app();
        let ctrl_c = KeyEvent::new_with_kind(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Release,
        );

        app.handle_key_event(ctrl_c).unwrap();

        assert_eq!(app.tasks.len(), 2);
        assert!(app.tasks.iter().all(|task| task.status == Status::Open));
    }

    #[test]
    fn test_selection_index_safety_on_filter() {
        let (mut app, _dir) = setup_app();
        app.selected_index = 1; // Select Banana

        // Start search for "Apple"
        app.handle_key_event(KeyCode::Char('/')).unwrap(); // Vim-like search
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
        app.handle_key_event(KeyCode::Char('/')).unwrap();
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
    fn test_attach_action_logic() {
        let (mut app, dir) = setup_app(); // Apple, Banana
        let root = dir.path();
        let file_path = "readme.md";
        std::fs::write(root.join(file_path), "content").unwrap();

        // Manually enter Attach mode
        app.input_mode = InputMode::Attach;
        app.input_buffer = String::new();

        // Type file path
        for c in file_path.chars() {
            app.handle_key_event(KeyCode::Char(c)).unwrap();
        }
        assert_eq!(app.input_buffer, file_path.to_string());

        // Press Enter
        app.handle_key_event(KeyCode::Enter).unwrap();

        // Should be back to Normal mode
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.input_buffer, "".to_string());

        // Apple (index 0) should now have the file linked
        assert_eq!(app.tasks[0].linked_files[0], file_path.to_string());
    }

    #[test]
    fn test_file_selection_toggle_logic() {
        let (mut app, dir) = setup_app(); // Apple, Banana
        let root = dir.path();
        let file_path = "readme.md";
        std::fs::write(root.join(file_path), "content").unwrap();

        // Enter FileSelect mode via 'A' (Shift-A)
        app.handle_key_event(KeyCode::Char('A')).unwrap();
        assert_eq!(app.input_mode, InputMode::FileSelect);
        assert!(app.project_files.contains(&file_path.to_string()));

        // Find index of readme.md
        let readme_idx = app
            .filtered_project_files
            .iter()
            .position(|f| f == file_path)
            .unwrap();
        app.file_selected_index = readme_idx;

        // Toggle attachment with Space
        app.handle_key_event(KeyCode::Char(' ')).unwrap();

        // Verify Apple (index 0) has file linked
        assert_eq!(app.tasks[0].linked_files[0], file_path.to_string());

        // Toggle again to detach
        app.handle_key_event(KeyCode::Char(' ')).unwrap();
        assert_eq!(app.tasks[0].linked_files.len(), 0);
    }

    #[test]
    fn test_file_search_logic() {
        let (mut app, dir) = setup_app();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("cargo.toml"), "content").unwrap();
        std::fs::write(root.join("src/main.rs"), "content").unwrap();

        app.handle_key_event(KeyCode::Char('A')).unwrap(); // Enter FileSelect
        assert!(app.filtered_project_files.len() >= 2);

        // Start search for "main"
        app.handle_key_event(KeyCode::Char('/')).unwrap();
        app.input_buffer = "main".to_string();
        app.refresh_tasks().unwrap();

        assert_eq!(app.filtered_project_files.len(), 1);
        assert!(app.filtered_project_files[0].contains("main.rs"));
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
        usecase
            .add_task("Very Long Task Title Indeed".to_string(), None, None)
            .unwrap();
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
        assert!(
            !buffer_str.contains("Indeed"),
            "Ghosting detected! 'Indeed' should have been cleared."
        );
    }
}
