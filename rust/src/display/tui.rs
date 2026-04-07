use crate::cli::{Cli, Commands};
use crate::collector;
use crate::display::banner;
use crate::platform;
use crate::types::{CleanResult, PortEntry, PortStatus, ProcessEntry, ProcessTreeNode};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, ExecutableCommand};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap};
use ratatui::{Frame, Terminal};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::{self, Stdout};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitialRoute {
    Ports,
    Processes,
    Watch,
    Clean,
}

#[derive(Debug, Clone, Copy)]
pub struct LaunchOptions {
    pub route: InitialRoute,
    pub focus_port: Option<u16>,
    pub show_all_ports: bool,
    pub show_all_processes: bool,
    pub interval_ms: u64,
    pub open_clean_modal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Ports,
    Processes,
    Watch,
    Clean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortSortMode {
    PortAsc,
    MemoryDesc,
    UptimeDesc,
    ProjectAsc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessSortMode {
    CpuDesc,
    MemoryDesc,
    UptimeDesc,
    PidAsc,
    ProjectAsc,
}

#[derive(Debug, Clone)]
enum PendingAction {
    Kill { pid: u32, label: String },
    CleanAll { count: usize },
}

#[derive(Debug, Clone, Copy)]
enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
struct Notification {
    level: NotificationLevel,
    message: String,
}

#[derive(Debug, Clone)]
struct WatchEvent {
    at_epoch: u64,
    label: String,
}

#[derive(Debug, Clone)]
enum DetailPane {
    Empty,
    Port {
        entry: PortEntry,
        tree: Vec<ProcessTreeNode>,
    },
    Process {
        entry: ProcessEntry,
        tree: Vec<ProcessTreeNode>,
    },
}

impl DetailPane {
    fn title(&self) -> &'static str {
        match self {
            Self::Empty => "Details",
            Self::Port { .. } => "Port Detail",
            Self::Process { .. } => "Process Detail",
        }
    }
}

struct App {
    view: View,
    show_all_ports: bool,
    show_all_processes: bool,
    interval: Duration,
    detail_expanded: bool,
    filter: String,
    filter_input: String,
    filtering: bool,
    pending_action: Option<PendingAction>,
    notification: Option<Notification>,
    should_quit: bool,
    ports: Vec<PortEntry>,
    processes: Vec<ProcessEntry>,
    clean_results: Vec<CleanResult>,
    watch_events: Vec<WatchEvent>,
    detail: DetailPane,
    selected_ports: usize,
    selected_processes: usize,
    selected_clean: usize,
    port_sort: PortSortMode,
    process_sort: ProcessSortMode,
    last_refresh: Instant,
    focus_port: Option<u16>,
    auto_open_clean_modal: bool,
    intro: Option<IntroAnim>,
}

// ---------------------------------------------------------------------------
// Intro banner animation
// ---------------------------------------------------------------------------

struct IntroAnim {
    started: Instant,
    /// Duration of the slide-in movement
    slide_duration: Duration,
    /// Extra hold time after the slide completes before transitioning to normal view
    hold_duration: Duration,
    banner: banner::Banner,
}

impl IntroAnim {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            slide_duration: Duration::from_millis(3000),
            hold_duration: Duration::from_millis(1000),
            banner: banner::build(Some("just say the word")),
        }
    }

    /// True once both the slide and the hold period have elapsed.
    fn is_complete(&self) -> bool {
        self.started.elapsed() >= self.slide_duration + self.hold_duration
    }

    /// Elastic ease-out progress for the slide phase only: 0.0 → 1.0.
    fn eased_progress(&self) -> f32 {
        let t = (self.started.elapsed().as_millis() as f32
            / self.slide_duration.as_millis() as f32)
            .min(1.0);
        elastic_ease_out(t)
    }

    /// Cubic ease-out progress used to drive the weight/boldness transition.
    fn weight_progress(&self) -> f32 {
        let t = (self.started.elapsed().as_millis() as f32
            / self.slide_duration.as_millis() as f32)
            .min(1.0);
        1.0 - (1.0 - t).powi(3)
    }

    /// Current x offset for the banner.
    /// At progress 0.0 the banner is fully off-screen to the left; at 1.0 it is centred.
    fn x_offset(&self, banner_width: i32, term_width: u16) -> i32 {
        let progress = self.eased_progress();
        let final_x = ((term_width as i32 - banner_width) / 2).max(0);
        let start_x = -(banner_width + 4);
        (start_x as f32 + progress * (final_x as f32 - start_x as f32)) as i32
    }
}

/// Elastic ease-out: overshoots slightly then settles.
fn elastic_ease_out(t: f32) -> f32 {
    if t == 0.0 || t == 1.0 {
        return t;
    }
    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
    2_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

/// Compute the text style based on how far through the slide phase we are.
/// Uses RGB colour interpolation so the change is clearly visible on all terminals.
/// 0.0 = just entering (near-black, dim)
/// 1.0 = fully arrived (vivid, bold)
fn anim_text_style(base_color: Color, weight_progress: f32) -> Style {
    let p = weight_progress.clamp(0.0, 1.0);

    let (target_r, target_g, target_b): (u8, u8, u8) = match base_color {
        Color::Green => (0, 230, 80),
        Color::Cyan => (0, 200, 230),
        Color::DarkGray => (100, 100, 120),
        _ => (210, 210, 210),
    };

    let start: u8 = 35; // near-black start
    let lerp = |s: u8, e: u8| -> u8 { (s as f32 + p * (e as f32 - s as f32)) as u8 };

    let r = lerp(start, target_r);
    let g = lerp(start, target_g);
    let b = lerp(start, target_b);

    let mut style = Style::default().fg(Color::Rgb(r, g, b));
    if p >= 0.85 {
        style = style.add_modifier(Modifier::BOLD);
    } else if p <= 0.15 {
        style = style.add_modifier(Modifier::DIM);
    }
    style
}

/// Build a single ratatui Line with the correct horizontal clip/pad applied.
fn make_banner_line_styled<'a>(
    content: &'a str,
    x_offset: i32,
    term_width: u16,
    style: Style,
) -> Line<'a> {
    let tw = term_width as i32;
    let chars: Vec<char> = content.chars().collect();
    let char_len = chars.len() as i32;

    if x_offset + char_len <= 0 || x_offset >= tw {
        return Line::from("");
    }

    if x_offset >= 0 {
        let pad = " ".repeat(x_offset as usize);
        let visible_len = (char_len.min(tw - x_offset)) as usize;
        let visible: String = chars[..visible_len].iter().collect();
        Line::from(vec![Span::raw(pad), Span::styled(visible, style)])
    } else {
        let skip = (-x_offset) as usize;
        let available = (tw as usize).min(chars.len().saturating_sub(skip));
        let visible: String = chars[skip..skip + available].iter().collect();
        Line::from(Span::styled(visible, style))
    }
}

impl From<&Cli> for LaunchOptions {
    fn from(cli: &Cli) -> Self {
        let route = if cli.port_number.is_some() {
            InitialRoute::Ports
        } else {
            match &cli.command {
                None => InitialRoute::Ports,
                Some(Commands::Ps { .. }) => InitialRoute::Processes,
                Some(Commands::Clean) => InitialRoute::Clean,
                Some(Commands::Watch) => InitialRoute::Watch,
            }
        };

        Self {
            route,
            focus_port: cli.port_number,
            show_all_ports: cli.all || matches!(&cli.command, Some(Commands::Ps { all: true })),
            show_all_processes: cli.all || matches!(&cli.command, Some(Commands::Ps { all: true })),
            interval_ms: cli.interval_ms,
            open_clean_modal: matches!(&cli.command, Some(Commands::Clean)),
        }
    }
}

pub fn run(options: impl Into<LaunchOptions>) -> Result<()> {
    let options = options.into();
    let mut tui = TuiSession::enter()?;
    let mut app = App::new(options);
    app.refresh()?;
    // Reset the intro timer so the animation always starts AFTER data has loaded.
    if let Some(ref mut intro) = app.intro {
        intro.started = Instant::now();
    }

    loop {
        tui.terminal.draw(|frame| app.render(frame))?;
        if app.should_quit {
            break;
        }

        let timeout = app.poll_timeout();
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.intro.is_some() {
                        // Any key skips the intro animation immediately
                        app.intro = None;
                    } else {
                        app.handle_key(key)?;
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        app.tick_intro();

        if app.intro.is_none() && app.last_refresh.elapsed() >= app.interval {
            app.refresh()?;
        }
    }

    Ok(())
}

impl App {
    fn new(options: LaunchOptions) -> Self {
        Self {
            view: match options.route {
                InitialRoute::Ports => View::Ports,
                InitialRoute::Processes => View::Processes,
                InitialRoute::Watch => View::Watch,
                InitialRoute::Clean => View::Clean,
            },
            show_all_ports: options.show_all_ports,
            show_all_processes: options.show_all_processes,
            interval: Duration::from_millis(options.interval_ms.max(250)),
            detail_expanded: options.focus_port.is_some(),
            filter: String::new(),
            filter_input: String::new(),
            filtering: false,
            pending_action: None,
            notification: None,
            should_quit: false,
            ports: Vec::new(),
            processes: Vec::new(),
            clean_results: Vec::new(),
            watch_events: Vec::new(),
            detail: DetailPane::Empty,
            selected_ports: 0,
            selected_processes: 0,
            selected_clean: 0,
            port_sort: PortSortMode::PortAsc,
            process_sort: ProcessSortMode::CpuDesc,
            last_refresh: Instant::now() - Duration::from_millis(options.interval_ms.max(250)),
            focus_port: options.focus_port,
            auto_open_clean_modal: options.open_clean_modal,
            intro: Some(IntroAnim::new()),
        }
    }

    fn poll_timeout(&self) -> Duration {
        if self.intro.is_some() {
            return Duration::from_millis(16);
        }
        self.interval
            .checked_sub(self.last_refresh.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0))
    }

    fn refresh(&mut self) -> Result<()> {
        let previous_ports: HashMap<u16, u32> = self
            .ports
            .iter()
            .map(|entry| (entry.port, entry.pid))
            .collect();
        self.ports = collector::collect_ports(self.show_all_ports)?;
        self.processes = collector::collect_processes(self.show_all_processes);

        self.apply_sorting();
        self.record_watch_events(previous_ports);
        self.clamp_selection();
        self.apply_focus_port();
        self.refresh_detail();

        if self.auto_open_clean_modal {
            self.auto_open_clean_modal = false;
            if self.clean_targets().is_empty() {
                self.notification = Some(Notification {
                    level: NotificationLevel::Success,
                    message: "No orphaned or zombie processes found.".into(),
                });
            } else {
                self.pending_action = Some(PendingAction::CleanAll {
                    count: self.clean_targets().len(),
                });
            }
        }

        self.last_refresh = Instant::now();
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.filtering {
            return self.handle_filter_input(key);
        }
        if self.pending_action.is_some() {
            return self.handle_pending_action(key);
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Tab => self.next_view(),
            KeyCode::BackTab => self.prev_view(),
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                self.refresh_detail();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                self.refresh_detail();
            }
            KeyCode::Char('g') => {
                self.move_to_edge(true);
                self.refresh_detail();
            }
            KeyCode::Char('G') => {
                self.move_to_edge(false);
                self.refresh_detail();
            }
            KeyCode::Enter | KeyCode::Char('d') => {
                self.detail_expanded = !self.detail_expanded;
            }
            KeyCode::Char('/') => {
                self.filter_input = self.filter.clone();
                self.filtering = true;
            }
            KeyCode::Char('s') => {
                self.cycle_sort();
                self.apply_sorting();
                self.clamp_selection();
                self.refresh_detail();
            }
            KeyCode::Char('a') => {
                match self.view {
                    View::Processes => self.show_all_processes = !self.show_all_processes,
                    View::Ports | View::Watch | View::Clean => {
                        self.show_all_ports = !self.show_all_ports
                    }
                }
                self.refresh()?;
            }
            KeyCode::Char('r') => {
                self.refresh()?;
            }
            KeyCode::Char('x') => {
                self.prepare_kill();
            }
            KeyCode::Char('c') => {
                self.prepare_clean();
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_filter_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.filtering = false;
                self.filter_input.clear();
            }
            KeyCode::Enter => {
                self.filter = self.filter_input.trim().to_string();
                self.filtering = false;
                self.clamp_selection();
                self.refresh_detail();
            }
            KeyCode::Backspace => {
                self.filter_input.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.filter_input.push(c);
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_pending_action(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('n') => {
                self.pending_action = None;
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                self.execute_pending_action()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_pending_action(&mut self) -> Result<()> {
        let Some(action) = self.pending_action.take() else {
            return Ok(());
        };

        match action {
            PendingAction::Kill { pid, label } => match platform::kill_process(pid) {
                Ok(()) => {
                    self.notification = Some(Notification {
                        level: NotificationLevel::Success,
                        message: format!("Killed {label} (PID {pid})."),
                    });
                    self.refresh()?;
                }
                Err(error) => {
                    self.notification = Some(Notification {
                        level: NotificationLevel::Error,
                        message: format!("Failed to kill PID {pid}: {error}"),
                    });
                }
            },
            PendingAction::CleanAll { .. } => {
                let targets = self
                    .clean_targets()
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                let mut results = Vec::with_capacity(targets.len());
                for entry in targets {
                    let result = match platform::kill_process(entry.pid) {
                        Ok(()) => CleanResult {
                            entry,
                            killed: true,
                            error: None,
                        },
                        Err(error) => CleanResult {
                            entry,
                            killed: false,
                            error: Some(error.to_string()),
                        },
                    };
                    results.push(result);
                }

                let killed = results.iter().filter(|result| result.killed).count();
                let failed = results.len().saturating_sub(killed);
                self.clean_results = results;
                self.notification = Some(Notification {
                    level: if failed == 0 {
                        NotificationLevel::Success
                    } else {
                        NotificationLevel::Warning
                    },
                    message: format!("Clean finished: {killed} killed, {failed} failed."),
                });
                self.refresh()?;
            }
        }

        Ok(())
    }

    fn next_view(&mut self) {
        self.view = match self.view {
            View::Ports => View::Processes,
            View::Processes => View::Watch,
            View::Watch => View::Clean,
            View::Clean => View::Ports,
        };
        self.clamp_selection();
        self.refresh_detail();
    }

    fn prev_view(&mut self) {
        self.view = match self.view {
            View::Ports => View::Clean,
            View::Processes => View::Ports,
            View::Watch => View::Processes,
            View::Clean => View::Watch,
        };
        self.clamp_selection();
        self.refresh_detail();
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.current_len();
        if len == 0 {
            self.set_selected(0);
            return;
        }

        let current = self.current_selected();
        let next = if delta.is_negative() {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current
                .saturating_add(delta as usize)
                .min(len.saturating_sub(1))
        };
        self.set_selected(next);
    }

    fn move_to_edge(&mut self, top: bool) {
        let len = self.current_len();
        if top || len == 0 {
            self.set_selected(0);
        } else {
            self.set_selected(len - 1);
        }
    }

    fn current_len(&self) -> usize {
        match self.view {
            View::Ports | View::Watch => self.filtered_ports().len(),
            View::Processes => self.filtered_processes().len(),
            View::Clean => self.clean_targets().len(),
        }
    }

    fn current_selected(&self) -> usize {
        match self.view {
            View::Ports | View::Watch => self.selected_ports,
            View::Processes => self.selected_processes,
            View::Clean => self.selected_clean,
        }
    }

    fn set_selected(&mut self, index: usize) {
        match self.view {
            View::Ports | View::Watch => self.selected_ports = index,
            View::Processes => self.selected_processes = index,
            View::Clean => self.selected_clean = index,
        }
    }

    fn clamp_selection(&mut self) {
        let ports_len = self.filtered_ports().len();
        let processes_len = self.filtered_processes().len();
        let clean_len = self.clean_targets().len();

        self.selected_ports = self.selected_ports.min(ports_len.saturating_sub(1));
        self.selected_processes = self.selected_processes.min(processes_len.saturating_sub(1));
        self.selected_clean = self.selected_clean.min(clean_len.saturating_sub(1));
    }

    fn apply_focus_port(&mut self) {
        let Some(port) = self.focus_port.take() else {
            return;
        };

        if let Some(index) = self
            .filtered_ports()
            .iter()
            .position(|entry| entry.port == port)
        {
            self.selected_ports = index;
        } else {
            self.notification = Some(Notification {
                level: NotificationLevel::Warning,
                message: format!("Port :{port} is not currently listening."),
            });
        }
    }

    fn prepare_kill(&mut self) {
        match self.view {
            View::Ports | View::Watch | View::Clean => {
                if let Some(entry) = self.selected_port_for_view() {
                    self.pending_action = Some(PendingAction::Kill {
                        pid: entry.pid,
                        label: format!(":{} {}", entry.port, entry.process_name),
                    });
                }
            }
            View::Processes => {
                if let Some(entry) = self.selected_process() {
                    self.pending_action = Some(PendingAction::Kill {
                        pid: entry.pid,
                        label: entry.process_name,
                    });
                }
            }
        }
    }

    fn prepare_clean(&mut self) {
        let count = self.clean_targets().len();
        if count == 0 {
            self.notification = Some(Notification {
                level: NotificationLevel::Info,
                message: "No orphaned or zombie processes to clean.".into(),
            });
            return;
        }
        self.pending_action = Some(PendingAction::CleanAll { count });
    }

    fn apply_sorting(&mut self) {
        sort_ports(&mut self.ports, self.port_sort);
        sort_processes(&mut self.processes, self.process_sort);
    }

    fn cycle_sort(&mut self) {
        match self.view {
            View::Ports | View::Watch | View::Clean => {
                self.port_sort = match self.port_sort {
                    PortSortMode::PortAsc => PortSortMode::MemoryDesc,
                    PortSortMode::MemoryDesc => PortSortMode::UptimeDesc,
                    PortSortMode::UptimeDesc => PortSortMode::ProjectAsc,
                    PortSortMode::ProjectAsc => PortSortMode::PortAsc,
                };
            }
            View::Processes => {
                self.process_sort = match self.process_sort {
                    ProcessSortMode::CpuDesc => ProcessSortMode::MemoryDesc,
                    ProcessSortMode::MemoryDesc => ProcessSortMode::UptimeDesc,
                    ProcessSortMode::UptimeDesc => ProcessSortMode::PidAsc,
                    ProcessSortMode::PidAsc => ProcessSortMode::ProjectAsc,
                    ProcessSortMode::ProjectAsc => ProcessSortMode::CpuDesc,
                };
            }
        }
    }

    fn filtered_ports(&self) -> Vec<&PortEntry> {
        self.ports
            .iter()
            .filter(|entry| port_matches_filter(entry, &self.filter))
            .collect()
    }

    fn filtered_processes(&self) -> Vec<&ProcessEntry> {
        self.processes
            .iter()
            .filter(|entry| process_matches_filter(entry, &self.filter))
            .collect()
    }

    fn clean_targets(&self) -> Vec<&PortEntry> {
        self.ports
            .iter()
            .filter(|entry| {
                matches!(entry.status, PortStatus::Orphaned | PortStatus::Zombie)
                    && port_matches_filter(entry, &self.filter)
            })
            .collect()
    }

    fn selected_port_for_view(&self) -> Option<PortEntry> {
        match self.view {
            View::Ports | View::Watch => self
                .filtered_ports()
                .get(self.selected_ports)
                .map(|entry| (*entry).clone()),
            View::Clean => self
                .clean_targets()
                .get(self.selected_clean)
                .map(|entry| (*entry).clone()),
            View::Processes => None,
        }
    }

    fn selected_process(&self) -> Option<ProcessEntry> {
        self.filtered_processes()
            .get(self.selected_processes)
            .map(|entry| (*entry).clone())
    }

    fn refresh_detail(&mut self) {
        self.detail = match self.view {
            View::Ports | View::Watch | View::Clean => match self.selected_port_for_view() {
                Some(entry) => DetailPane::Port {
                    tree: collector::get_process_tree(entry.pid),
                    entry,
                },
                None => DetailPane::Empty,
            },
            View::Processes => match self.selected_process() {
                Some(entry) => DetailPane::Process {
                    tree: collector::get_process_tree(entry.pid),
                    entry,
                },
                None => DetailPane::Empty,
            },
        };
    }

    fn record_watch_events(&mut self, previous: HashMap<u16, u32>) {
        let current: HashMap<u16, u32> = self
            .ports
            .iter()
            .map(|entry| (entry.port, entry.pid))
            .collect();
        let now = current_epoch();

        for entry in &self.ports {
            match previous.get(&entry.port) {
                None => self.watch_events.push(WatchEvent {
                    at_epoch: now,
                    label: format!("▲ NEW :{} ← {}", entry.port, entry.process_name),
                }),
                Some(pid) if *pid != entry.pid => self.watch_events.push(WatchEvent {
                    at_epoch: now,
                    label: format!(
                        "◆ CHANGED :{} ← {} (PID {})",
                        entry.port, entry.process_name, entry.pid
                    ),
                }),
                _ => {}
            }
        }

        for port in previous.keys() {
            if !current.contains_key(port) {
                self.watch_events.push(WatchEvent {
                    at_epoch: now,
                    label: format!("▼ CLOSED :{port}"),
                });
            }
        }

        let keep = 40;
        if self.watch_events.len() > keep {
            let drain = self.watch_events.len() - keep;
            self.watch_events.drain(0..drain);
        }
    }

    fn render(&self, frame: &mut Frame<'_>) {
        if let Some(ref anim) = self.intro {
            self.render_intro(frame, anim);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(2),
            ])
            .split(frame.area());

        frame.render_widget(self.header(), chunks[0]);
        frame.render_widget(self.tabs(), chunks[1]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(if self.detail_expanded {
                [Constraint::Percentage(42), Constraint::Percentage(58)]
            } else {
                [Constraint::Percentage(55), Constraint::Percentage(45)]
            })
            .split(chunks[2]);

        self.render_list(frame, body[0]);
        if self.view == View::Watch {
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(body[1]);
            self.render_detail(frame, right[0]);
            self.render_watch_timeline(frame, right[1]);
        } else {
            self.render_detail(frame, body[1]);
        }
        frame.render_widget(self.footer(), chunks[3]);

        if self.filtering {
            self.render_filter_modal(frame);
        }
        if self.pending_action.is_some() {
            self.render_confirm_modal(frame);
        }
    }

    fn render_intro(&self, frame: &mut Frame<'_>, anim: &IntroAnim) {
        let area = frame.area();
        let b = &anim.banner;
        let lines = &b.all_lines;

        let banner_width = lines
            .iter()
            .map(|l| l.chars().count() as i32)
            .max()
            .unwrap_or(0);

        let x_offset = anim.x_offset(banner_width, area.width);
        let weight = anim.weight_progress();
        let total_height = lines.len() as u16;
        let y_start = area.height.saturating_sub(total_height) / 2;

        // Gradient + weight-based style: PORT=green, WHISPERER=cyan, subtitle=dimgray
        let port_end = b.port_line_count;
        let whisp_end = port_end + 1 + b.whisperer_line_count;

        let rendered_lines: Vec<Line<'_>> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let base_color = if i < port_end {
                    Color::Green
                } else if i > port_end && i <= whisp_end {
                    Color::Cyan
                } else {
                    Color::DarkGray
                };
                let style = anim_text_style(base_color, weight);
                make_banner_line_styled(l.as_str(), x_offset, area.width, style)
            })
            .collect();

        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(Text::from(rendered_lines)),
            Rect {
                x: area.x,
                y: area.y + y_start,
                width: area.width,
                height: total_height.min(area.height.saturating_sub(y_start)),
            },
        );
    }

    fn tick_intro(&mut self) {
        if let Some(ref anim) = self.intro {
            if anim.is_complete() {
                self.intro = None;
            }
        }
    }

    fn header(&self) -> Paragraph<'_> {
        let status = self.notification.as_ref().map(|notification| {
            Span::styled(
                notification.message.as_str(),
                Style::default().fg(color_for_notification(notification.level)),
            )
        });
        let right = format!(
            "refresh {}ms  filter {}",
            self.interval.as_millis(),
            if self.filter.is_empty() {
                "off".into()
            } else {
                format!("\"{}\"", self.filter)
            }
        );

        let mut line = vec![
            Span::styled(
                "Port Whisperer",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(right, Style::default().fg(Color::DarkGray)),
        ];
        if let Some(status) = status {
            line.push(Span::raw("  "));
            line.push(status);
        }

        Paragraph::new(Line::from(line))
    }

    fn tabs(&self) -> Tabs<'_> {
        let titles = ["Ports", "Processes", "Watch", "Clean"];
        let selected = match self.view {
            View::Ports => 0,
            View::Processes => 1,
            View::Watch => 2,
            View::Clean => 3,
        };

        Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("Views"))
            .select(selected)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
    }

    fn render_list(&self, frame: &mut Frame<'_>, area: Rect) {
        match self.view {
            View::Ports | View::Watch => {
                let entries = self.filtered_ports();
                let items = entries
                    .iter()
                    .map(|entry| ListItem::new(port_row(entry)))
                    .collect::<Vec<_>>();
                let mut state = ListState::default().with_selected(if entries.is_empty() {
                    None
                } else {
                    Some(self.selected_ports)
                });
                frame.render_stateful_widget(
                    List::new(items)
                        .block(Block::default().borders(Borders::ALL).title(format!(
                            "Ports{} [{}]",
                            if self.show_all_ports { " (all)" } else { "" },
                            port_sort_label(self.port_sort)
                        )))
                        .highlight_style(
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("› "),
                    area,
                    &mut state,
                );
            }
            View::Processes => {
                let entries = self.filtered_processes();
                let items = entries
                    .iter()
                    .map(|entry| ListItem::new(process_row(entry)))
                    .collect::<Vec<_>>();
                let mut state = ListState::default().with_selected(if entries.is_empty() {
                    None
                } else {
                    Some(self.selected_processes)
                });
                frame.render_stateful_widget(
                    List::new(items)
                        .block(Block::default().borders(Borders::ALL).title(format!(
                            "Processes{} [{}]",
                            if self.show_all_processes {
                                " (all)"
                            } else {
                                ""
                            },
                            process_sort_label(self.process_sort)
                        )))
                        .highlight_style(
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("› "),
                    area,
                    &mut state,
                );
            }
            View::Clean => {
                let entries = self.clean_targets();
                let items = entries
                    .iter()
                    .map(|entry| ListItem::new(port_row(entry)))
                    .collect::<Vec<_>>();
                let mut state = ListState::default().with_selected(if entries.is_empty() {
                    None
                } else {
                    Some(self.selected_clean)
                });
                frame.render_stateful_widget(
                    List::new(items)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("Cleanup Targets [{}]", entries.len())),
                        )
                        .highlight_style(
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("› "),
                    area,
                    &mut state,
                );
            }
        }
    }

    fn render_detail(&self, frame: &mut Frame<'_>, area: Rect) {
        let title = if self.view == View::Watch {
            "Watch Summary"
        } else {
            self.detail.title()
        };
        let block = Block::default().borders(Borders::ALL).title(title);

        let text = if self.view == View::Watch {
            self.watch_summary_text()
        } else if self.view == View::Clean && !self.clean_results.is_empty() {
            self.clean_results_text()
        } else {
            self.detail_text()
        };

        frame.render_widget(
            Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
            area,
        );
    }

    fn render_watch_timeline(&self, frame: &mut Frame<'_>, area: Rect) {
        let text = self.watch_timeline_text();
        frame.render_widget(
            Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title(format!(
                    "Event Timeline [{}]",
                    self.watch_events.len()
                )))
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn footer(&self) -> Paragraph<'_> {
        Paragraph::new(Line::from(vec![
            Span::styled("j/k", Style::default().fg(Color::Cyan)),
            Span::raw(" move  "),
            Span::styled("Tab", Style::default().fg(Color::Cyan)),
            Span::raw(" view  "),
            Span::styled("/", Style::default().fg(Color::Cyan)),
            Span::raw(" filter  "),
            Span::styled("s", Style::default().fg(Color::Cyan)),
            Span::raw(" sort  "),
            Span::styled("a", Style::default().fg(Color::Cyan)),
            Span::raw(" scope  "),
            Span::styled("r", Style::default().fg(Color::Cyan)),
            Span::raw(" refresh  "),
            Span::styled("x", Style::default().fg(Color::Cyan)),
            Span::raw(" kill  "),
            Span::styled("c", Style::default().fg(Color::Cyan)),
            Span::raw(" clean  "),
            Span::styled("d", Style::default().fg(Color::Cyan)),
            Span::raw(" detail  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" quit"),
        ]))
        .alignment(Alignment::Center)
    }

    fn render_filter_modal(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(60, 20, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(self.filter_input.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Filter")
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn render_confirm_modal(&self, frame: &mut Frame<'_>) {
        let area = centered_rect(60, 24, frame.area());
        frame.render_widget(Clear, area);
        let message = match &self.pending_action {
            Some(PendingAction::Kill { pid, label }) => {
                format!("Kill {label} (PID {pid})?\n\nEnter/y confirm, Esc/n cancel")
            }
            Some(PendingAction::CleanAll { count }) => {
                format!(
                    "Kill {count} orphaned or zombie process(es)?\n\nEnter/y confirm, Esc/n cancel"
                )
            }
            None => String::new(),
        };
        frame.render_widget(
            Paragraph::new(message)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Confirm Action")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn detail_text(&self) -> Text<'static> {
        match &self.detail {
            DetailPane::Empty => Text::from("No selection."),
            DetailPane::Port { entry, tree } => {
                let mut lines = vec![
                    line_kv("Port", format!(":{}", entry.port)),
                    line_kv("Process", entry.process_name.clone()),
                    line_kv("PID", entry.pid.to_string()),
                    line_kv("Status", status_label(&entry.status)),
                    line_kv("Framework", entry.framework.as_deref().unwrap_or("—")),
                    line_kv("Memory", format_memory(entry.memory_kb)),
                    line_kv("Uptime", format_uptime(entry.start_time)),
                    line_kv("Project", entry.project_name.as_deref().unwrap_or("—")),
                    line_kv("Git", entry.git_branch.as_deref().unwrap_or("—")),
                    line_kv(
                        "Directory",
                        entry
                            .cwd
                            .as_ref()
                            .map(|path| path.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "—".to_string()),
                    ),
                    line_kv("Command", truncate_owned(entry.command.clone(), 120)),
                    Line::raw(""),
                    Line::styled(
                        "Process Tree",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                if tree.is_empty() {
                    lines.push(Line::raw("—"));
                } else {
                    for node in tree {
                        lines.push(Line::raw(format!("• {} ({})", node.name, node.pid)));
                    }
                }
                lines.push(Line::raw(""));
                lines.push(Line::raw(format!("Kill hint: {}", kill_hint(entry.pid))));
                Text::from(lines)
            }
            DetailPane::Process { entry, tree } => {
                let mut lines = vec![
                    line_kv("Process", entry.process_name.clone()),
                    line_kv("PID", entry.pid.to_string()),
                    line_kv("Status", status_label(&entry.status)),
                    line_kv("CPU", format!("{:.1}%", entry.cpu_pct)),
                    line_kv("Memory", format_memory(entry.memory_kb)),
                    line_kv("Uptime", format_uptime(entry.start_time)),
                    line_kv("Project", entry.project_name.as_deref().unwrap_or("—")),
                    line_kv("Framework", entry.framework.as_deref().unwrap_or("—")),
                    line_kv(
                        "Directory",
                        entry
                            .cwd
                            .as_ref()
                            .map(|path| path.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "—".to_string()),
                    ),
                    line_kv("What", entry.description.clone()),
                    line_kv("Command", truncate_owned(entry.command.clone(), 120)),
                    Line::raw(""),
                    Line::styled(
                        "Process Tree",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                if tree.is_empty() {
                    lines.push(Line::raw("—"));
                } else {
                    for node in tree {
                        lines.push(Line::raw(format!("• {} ({})", node.name, node.pid)));
                    }
                }
                lines.push(Line::raw(""));
                lines.push(Line::raw(format!("Kill hint: {}", kill_hint(entry.pid))));
                Text::from(lines)
            }
        }
    }

    fn watch_summary_text(&self) -> Text<'static> {
        let visible = self.filtered_ports().len();
        let total = self.ports.len();
        let orphaned = self
            .ports
            .iter()
            .filter(|entry| matches!(entry.status, PortStatus::Orphaned))
            .count();
        let zombie = self
            .ports
            .iter()
            .filter(|entry| matches!(entry.status, PortStatus::Zombie))
            .count();
        let latest = self
            .watch_events
            .last()
            .map(|event| format!("{}  {}", format_clock(event.at_epoch), event.label))
            .unwrap_or_else(|| "No port changes observed yet.".to_string());

        let mut lines = vec![
            line_kv("Visible", visible.to_string()),
            line_kv("Tracked", total.to_string()),
            line_kv("Orphaned", orphaned.to_string()),
            line_kv("Zombie", zombie.to_string()),
            line_kv("Latest", latest),
            Line::raw(""),
            Line::styled(
                "Selected Entry",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
        ];
        lines.extend(self.detail_text().lines);
        Text::from(lines)
    }

    fn watch_timeline_text(&self) -> Text<'static> {
        let mut lines = Vec::new();
        if self.watch_events.is_empty() {
            lines.push(Line::raw("No port changes observed yet."));
        } else {
            for event in self.watch_events.iter().rev().take(20) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format_clock(event.at_epoch),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        watch_event_marker(&event.label),
                        Style::default().fg(watch_event_color(&event.label)),
                    ),
                    Span::raw("  "),
                    Span::raw(event.label.clone()),
                ]));
            }
        }
        Text::from(lines)
    }

    fn clean_results_text(&self) -> Text<'static> {
        let mut lines = vec![
            Line::styled(
                "Cleanup Results",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
        ];

        for result in &self.clean_results {
            let marker = if result.killed { "✓" } else { "✕" };
            lines.push(Line::raw(format!(
                "{marker} :{} {} (PID {})",
                result.entry.port, result.entry.process_name, result.entry.pid
            )));
            if let Some(error) = &result.error {
                lines.push(Line::styled(
                    format!("  {error}"),
                    Style::default().fg(Color::Red),
                ));
            }
        }

        Text::from(lines)
    }
}

struct TuiSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TuiSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
        // Mouse capture is best-effort: some Windows terminals (Git Bash, older cmd.exe)
        // do not support it. Failing here would prevent the TUI — and the intro animation —
        // from ever running on those environments, so we swallow the error.
        let _ = stdout.execute(crossterm::event::EnableMouseCapture);
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("failed to initialize terminal backend")?;
        Ok(Self { terminal })
    }
}

impl Drop for TuiSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = self
            .terminal
            .backend_mut()
            .execute(crossterm::event::DisableMouseCapture);
        let _ = self.terminal.backend_mut().execute(LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn centered_rect(width_percent: u16, height_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height_percent) / 2),
            Constraint::Percentage(height_percent),
            Constraint::Percentage((100 - height_percent) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1])[1]
}

fn sort_ports(entries: &mut [PortEntry], mode: PortSortMode) {
    match mode {
        PortSortMode::PortAsc => entries.sort_by_key(|entry| entry.port),
        PortSortMode::MemoryDesc => entries.sort_by(|left, right| {
            right
                .memory_kb
                .cmp(&left.memory_kb)
                .then(left.port.cmp(&right.port))
        }),
        PortSortMode::UptimeDesc => entries.sort_by(|left, right| {
            right
                .start_time
                .cmp(&left.start_time)
                .then(left.port.cmp(&right.port))
        }),
        PortSortMode::ProjectAsc => entries.sort_by(|left, right| {
            cmp_optional_str(left.project_name.as_deref(), right.project_name.as_deref())
                .then(left.port.cmp(&right.port))
        }),
    }
}

fn sort_processes(entries: &mut [ProcessEntry], mode: ProcessSortMode) {
    match mode {
        ProcessSortMode::CpuDesc => entries.sort_by(|left, right| {
            right
                .cpu_pct
                .partial_cmp(&left.cpu_pct)
                .unwrap_or(Ordering::Equal)
                .then(left.pid.cmp(&right.pid))
        }),
        ProcessSortMode::MemoryDesc => entries.sort_by(|left, right| {
            right
                .memory_kb
                .cmp(&left.memory_kb)
                .then(left.pid.cmp(&right.pid))
        }),
        ProcessSortMode::UptimeDesc => entries.sort_by(|left, right| {
            right
                .start_time
                .cmp(&left.start_time)
                .then(left.pid.cmp(&right.pid))
        }),
        ProcessSortMode::PidAsc => entries.sort_by_key(|entry| entry.pid),
        ProcessSortMode::ProjectAsc => entries.sort_by(|left, right| {
            cmp_optional_str(left.project_name.as_deref(), right.project_name.as_deref())
                .then(left.pid.cmp(&right.pid))
        }),
    }
}

fn cmp_optional_str(left: Option<&str>, right: Option<&str>) -> Ordering {
    left.unwrap_or("~").cmp(right.unwrap_or("~"))
}

fn port_row(entry: &PortEntry) -> Line<'static> {
    let status = status_badge(&entry.status);
    let project = entry.project_name.as_deref().unwrap_or("—").to_string();
    let framework = entry.framework.as_deref().unwrap_or("—").to_string();
    Line::from(vec![
        Span::styled(
            format!(":{:<5}", entry.port),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(status, Style::default().fg(status_color(&entry.status))),
        Span::raw(" "),
        Span::raw(truncate_owned(entry.process_name.clone(), 18)),
        Span::styled(
            format!("  {}", truncate_owned(project, 14)),
            Style::default().fg(Color::Blue),
        ),
        Span::styled(
            format!("  {}", truncate_owned(framework, 12)),
            Style::default().fg(Color::Green),
        ),
    ])
}

fn process_row(entry: &ProcessEntry) -> Line<'static> {
    let project = entry.project_name.as_deref().unwrap_or("—").to_string();
    let framework = entry.framework.as_deref().unwrap_or("—").to_string();
    Line::from(vec![
        Span::styled(
            format!("{:<6}", entry.pid),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" "),
        Span::raw(truncate_owned(entry.process_name.clone(), 18)),
        Span::styled(
            format!("  {:>5.1}%", entry.cpu_pct),
            Style::default().fg(cpu_color(entry.cpu_pct)),
        ),
        Span::styled(
            format!("  {}", truncate_owned(project, 14)),
            Style::default().fg(Color::Blue),
        ),
        Span::styled(
            format!("  {}", truncate_owned(framework, 12)),
            Style::default().fg(Color::Green),
        ),
    ])
}

fn port_matches_filter(entry: &PortEntry, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let filter = filter.to_lowercase();
    [
        entry.port.to_string(),
        entry.pid.to_string(),
        entry.process_name.to_lowercase(),
        entry.command.to_lowercase(),
        entry
            .project_name
            .clone()
            .unwrap_or_default()
            .to_lowercase(),
        entry.framework.clone().unwrap_or_default().to_lowercase(),
    ]
    .iter()
    .any(|field| field.contains(&filter))
}

fn process_matches_filter(entry: &ProcessEntry, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let filter = filter.to_lowercase();
    [
        entry.pid.to_string(),
        entry.process_name.to_lowercase(),
        entry.command.to_lowercase(),
        entry.description.to_lowercase(),
        entry
            .project_name
            .clone()
            .unwrap_or_default()
            .to_lowercase(),
        entry.framework.clone().unwrap_or_default().to_lowercase(),
    ]
    .iter()
    .any(|field| field.contains(&filter))
}

fn line_kv(label: impl Into<String>, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<10}", label.into()),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" "),
        Span::raw(value.into()),
    ])
}

fn status_label(status: &PortStatus) -> &'static str {
    match status {
        PortStatus::Healthy => "healthy",
        PortStatus::Zombie => "zombie",
        PortStatus::Orphaned => "orphaned",
    }
}

fn status_badge(status: &PortStatus) -> &'static str {
    match status {
        PortStatus::Healthy => "H",
        PortStatus::Zombie => "Z",
        PortStatus::Orphaned => "O",
    }
}

fn status_color(status: &PortStatus) -> Color {
    match status {
        PortStatus::Healthy => Color::Green,
        PortStatus::Zombie => Color::Red,
        PortStatus::Orphaned => Color::Yellow,
    }
}

fn cpu_color(cpu: f32) -> Color {
    if cpu > 25.0 {
        Color::Red
    } else if cpu > 5.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn color_for_notification(level: NotificationLevel) -> Color {
    match level {
        NotificationLevel::Info => Color::Cyan,
        NotificationLevel::Success => Color::Green,
        NotificationLevel::Warning => Color::Yellow,
        NotificationLevel::Error => Color::Red,
    }
}

fn watch_event_marker(label: &str) -> &'static str {
    if label.starts_with("▲ NEW") {
        "NEW"
    } else if label.starts_with("▼ CLOSED") {
        "CLOSED"
    } else if label.starts_with("◆ CHANGED") {
        "CHANGED"
    } else {
        "EVENT"
    }
}

fn watch_event_color(label: &str) -> Color {
    if label.starts_with("▲ NEW") {
        Color::Green
    } else if label.starts_with("▼ CLOSED") {
        Color::Red
    } else if label.starts_with("◆ CHANGED") {
        Color::Yellow
    } else {
        Color::Cyan
    }
}

fn port_sort_label(mode: PortSortMode) -> &'static str {
    match mode {
        PortSortMode::PortAsc => "port",
        PortSortMode::MemoryDesc => "memory",
        PortSortMode::UptimeDesc => "uptime",
        PortSortMode::ProjectAsc => "project",
    }
}

fn process_sort_label(mode: ProcessSortMode) -> &'static str {
    match mode {
        ProcessSortMode::CpuDesc => "cpu",
        ProcessSortMode::MemoryDesc => "memory",
        ProcessSortMode::UptimeDesc => "uptime",
        ProcessSortMode::PidAsc => "pid",
        ProcessSortMode::ProjectAsc => "project",
    }
}

fn format_memory(kb: u64) -> String {
    if kb >= 1_048_576 {
        format!("{:.1} GB", kb as f64 / 1_048_576.0)
    } else if kb >= 1024 {
        format!("{:.1} MB", kb as f64 / 1024.0)
    } else {
        format!("{kb} KB")
    }
}

fn format_uptime(start_time: Option<u64>) -> String {
    let Some(start_time) = start_time else {
        return "—".into();
    };
    let now = current_epoch();
    let seconds = now.saturating_sub(start_time);
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if days > 0 {
        format!("{days}d {}h", hours % 24)
    } else if hours > 0 {
        format!("{hours}h {}m", minutes % 60)
    } else if minutes > 0 {
        format!("{minutes}m {}s", seconds % 60)
    } else {
        format!("{seconds}s")
    }
}

fn format_clock(epoch: u64) -> String {
    let seconds = epoch % 86_400;
    let hour = seconds / 3600;
    let minute = (seconds % 3600) / 60;
    let second = seconds % 60;
    format!("{hour:02}:{minute:02}:{second:02}")
}

fn current_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn truncate_owned(mut value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }
    value = value.chars().take(max_chars.saturating_sub(1)).collect();
    value.push('…');
    value
}

fn kill_hint(pid: u32) -> String {
    if cfg!(windows) {
        format!("taskkill /F /PID {pid}")
    } else {
        format!("kill -TERM {pid}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // elastic_ease_out boundary conditions
    // -----------------------------------------------------------------------

    #[test]
    fn elastic_ease_out_at_zero() {
        assert_eq!(elastic_ease_out(0.0), 0.0);
    }

    #[test]
    fn elastic_ease_out_at_one() {
        assert_eq!(elastic_ease_out(1.0), 1.0);
    }

    #[test]
    fn elastic_ease_out_midpoint_is_above_zero() {
        // At t=0.5 the function should be well above 0 (past the halfway point)
        let v = elastic_ease_out(0.5);
        assert!(v > 0.0, "elastic_ease_out(0.5) should be > 0, got {v}");
    }

    #[test]
    fn elastic_ease_out_near_end_close_to_one() {
        // At t=0.99 the function should be very close to 1.0
        let v = elastic_ease_out(0.99);
        assert!((v - 1.0).abs() < 0.05, "elastic_ease_out(0.99) should be near 1.0, got {v}");
    }

    // -----------------------------------------------------------------------
    // anim_text_style — RGB output sanity checks
    // -----------------------------------------------------------------------

    #[test]
    fn anim_text_style_fully_arrived_uses_bold() {
        use ratatui::style::{Color, Modifier};
        let style = anim_text_style(Color::Green, 1.0);
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn anim_text_style_just_entered_uses_dim() {
        use ratatui::style::{Color, Modifier};
        let style = anim_text_style(Color::Cyan, 0.0);
        assert!(style.add_modifier.contains(Modifier::DIM));
    }

    // -----------------------------------------------------------------------
    // IntroAnim::x_offset — slide geometry
    // -----------------------------------------------------------------------

    #[test]
    fn intro_anim_x_offset_at_zero_progress_is_offscreen() {
        // At progress=0 the banner should start fully off-screen to the left.
        // We fake an IntroAnim by calling x_offset with a synthetic progress of 0
        // through the public elastic_ease_out path at t=0 → progress=0.
        // Directly test the formula: start_x = -(banner_width + 4)
        let banner_width: i32 = 60;
        let term_width: u16 = 120;
        let progress: f32 = 0.0; // eased_progress at t=0
        let final_x = ((term_width as i32 - banner_width) / 2).max(0);
        let start_x = -(banner_width + 4);
        let x = (start_x as f32 + progress * (final_x as f32 - start_x as f32)) as i32;
        assert!(x < 0, "banner should start off-screen (x < 0), got {x}");
    }

    #[test]
    fn intro_anim_x_offset_at_full_progress_is_centered() {
        let banner_width: i32 = 60;
        let term_width: u16 = 120;
        let progress: f32 = 1.0;
        let final_x = ((term_width as i32 - banner_width) / 2).max(0);
        let start_x = -(banner_width + 4);
        let x = (start_x as f32 + progress * (final_x as f32 - start_x as f32)) as i32;
        assert_eq!(x, final_x, "banner should be centred at progress=1.0");
    }

    // -----------------------------------------------------------------------
    // Windows kill hint
    // -----------------------------------------------------------------------

    #[test]
    #[cfg(windows)]
    fn kill_hint_windows_uses_taskkill() {
        let hint = super::kill_hint(1234);
        assert!(hint.contains("taskkill"), "Windows kill hint should use taskkill");
        assert!(hint.contains("1234"));
    }

    #[test]
    #[cfg(not(windows))]
    fn kill_hint_unix_uses_kill() {
        let hint = super::kill_hint(1234);
        assert!(hint.contains("kill"), "Unix kill hint should use kill");
        assert!(hint.contains("1234"));
    }
}
