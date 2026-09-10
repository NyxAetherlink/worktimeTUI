mod model;
mod storage;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseButton, MouseEventKind,
    },
    execute,
};
use model::{Data, Project, Timer, duration};
use ratatui::{prelude::*, widgets::*};
use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};
use storage::Store;

const BG: Color = Color::Rgb(46, 52, 64);
const CYAN: Color = Color::Rgb(0, 229, 255);
const BLUE: Color = Color::Rgb(0, 119, 182);
const WHITE: Color = Color::Rgb(229, 233, 240);
const GREEN: Color = Color::Rgb(163, 190, 140);
const MAGENTA: Color = Color::Rgb(180, 142, 173);

#[derive(Default)]
struct App {
    data: Data,
    selected: usize,
    timer: Timer,
    input: Option<String>,
    message: String,
    list: ListState,
    project_area: Rect,
    button_area: Rect,
}
impl App {
    fn select(&mut self, index: usize) {
        if index != self.selected && index < self.data.projects.len() {
            self.timer = Timer::default();
            self.selected = index;
            self.message = "Project opened. Previous timer stopped and saved.".into();
        }
    }
    fn toggle(&mut self) {
        if self.data.projects.is_empty() {
            self.message = "Press N to create your first project.".into();
        } else {
            self.timer.running = !self.timer.running;
            self.message.clear();
        }
    }
    fn key(&mut self, key: event::KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return true;
        }
        if let Some(input) = &mut self.input {
            match key.code {
                KeyCode::Esc => self.input = None,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) if !c.is_control() && input.chars().count() < 64 => input.push(c),
                KeyCode::Enter => {
                    let name = input.trim().to_string();
                    if name.is_empty() {
                        self.message = "Give your project a name.".into();
                    } else if self
                        .data
                        .projects
                        .iter()
                        .any(|p| p.name.to_lowercase() == name.to_lowercase())
                    {
                        self.message = "That project already exists. Choose another name.".into();
                    } else {
                        self.data.projects.push(Project {
                            name,
                            ..Default::default()
                        });
                        self.select(self.data.projects.len() - 1);
                        self.input = None;
                        self.message = "Project created. Space starts the timer.".into();
                    }
                }
                _ => (),
            }
            return false;
        }
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Char('n' | 'N') => self.input = Some(String::new()),
            KeyCode::Char(' ') | KeyCode::Enter => self.toggle(),
            KeyCode::Down | KeyCode::Char('j') => {
                self.select((self.selected + 1).min(self.data.projects.len().saturating_sub(1)))
            }
            KeyCode::Up | KeyCode::Char('k') => self.select(self.selected.saturating_sub(1)),
            KeyCode::Char('p') => {
                self.timer = Timer::default();
                self.data.pomodoro = !self.data.pomodoro;
                self.message = "Mode changed. Timer stopped; recorded time retained.".into();
            }
            KeyCode::Char('b') => self.data.include_breaks = !self.data.include_breaks,
            KeyCode::Char('s') => {
                self.timer = Timer::default();
                self.message = "Session stopped. All recorded time retained.".into();
            }
            _ => (),
        }
        false
    }
    fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(
            Block::default().style(Style::default().bg(BG).fg(WHITE)),
            area,
        );
        self.project_area = Rect::default();
        self.button_area = Rect::default();
        if area.width < 65 || area.height < 27 {
            f.render_widget(Paragraph::new("worktimeTUI\nResize terminal to at least 65 × 27.\nTimer continues. Space: pause · Q: save & quit").wrap(Wrap { trim: false }), area);
            return;
        }
        let rows = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(4),
        ])
        .split(area);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ◈ worktimeTUI ", Style::default().fg(CYAN).bold()),
                Span::raw(" / CYBERNORD     FOCUSED • MINIMAL • RELENTLESS"),
            ]))
            .block(Block::bordered().border_style(Style::default().fg(BLUE))),
            rows[0],
        );
        let columns = Layout::horizontal([Constraint::Percentage(34), Constraint::Percentage(66)])
            .split(rows[1]);
        self.project_area = columns[0];
        self.list.select(if self.data.projects.is_empty() {
            None
        } else {
            Some(self.selected)
        });
        let items: Vec<ListItem> = self
            .data
            .projects
            .iter()
            .map(|p| {
                ListItem::new(format!(
                    "{}  {}",
                    p.name,
                    duration(
                        p.focus_ms
                            + if self.data.include_breaks {
                                p.break_ms
                            } else {
                                0
                            }
                    )
                ))
            })
            .collect();
        f.render_stateful_widget(
            List::new(items)
                .block(panel(" PROJECTS / N new "))
                .highlight_symbol("▸ ")
                .highlight_style(Style::default().bg(BLUE).fg(CYAN).bold()),
            columns[0],
            &mut self.list,
        );
        let right = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(columns[1]);
        let project = self.data.projects.get(self.selected);
        f.render_widget(
            Paragraph::new(
                project
                    .map(|p| p.name.as_str())
                    .unwrap_or("Create a project with N to begin"),
            )
            .style(Style::default().fg(CYAN).bold())
            .block(panel(" CURRENT PROJECT ")),
            right[0],
        );
        let phase = if self.data.pomodoro {
            if self.timer.on_break {
                "BREAK"
            } else {
                "FOCUS"
            }
        } else {
            "STOPWATCH"
        };
        let state = if self.timer.running {
            "RUNNING"
        } else {
            "PAUSED"
        };
        let clock = if self.data.pomodoro {
            self.timer.limit() - self.timer.phase_ms
        } else {
            self.timer.session_ms
        };
        let text = vec![
            Line::from(format!("{phase} / {state}"))
                .style(Style::default().fg(if self.timer.on_break { MAGENTA } else { GREEN })),
            Line::from(""),
            Line::from(duration(clock)).style(Style::default().fg(CYAN).bold()),
            Line::from(""),
            Line::from(format!("Completed focus rounds: {}", self.timer.rounds)),
        ];
        f.render_widget(
            Paragraph::new(text)
                .alignment(Alignment::Center)
                .block(panel(" TIMER ")),
            right[1],
        );
        self.button_area = right[2];
        f.render_widget(
            Paragraph::new(if self.timer.running {
                "⏸  PAUSE / SPACE"
            } else {
                "▶  START / SPACE"
            })
            .alignment(Alignment::Center)
            .style(Style::default().fg(CYAN).bold())
            .block(panel(" CLICK ")),
            right[2],
        );
        let stats = if let Some(p) = project {
            format!(
                "Tracked total   {}\nFocus           {}\nBreaks          {}\nBreak accounting: {}\nMode: {}",
                duration(
                    p.focus_ms
                        + if self.data.include_breaks {
                            p.break_ms
                        } else {
                            0
                        }
                ),
                duration(p.focus_ms),
                duration(p.break_ms),
                if self.data.include_breaks {
                    "INCLUDED"
                } else {
                    "EXCLUDED"
                },
                if self.data.pomodoro {
                    "Pomodoro 25 / 5 / 15"
                } else {
                    "Stopwatch"
                }
            )
        } else {
            "Your projects and totals are saved automatically.\nPress N to create a project.".into()
        };
        f.render_widget(
            Paragraph::new(stats)
                .wrap(Wrap { trim: false })
                .block(panel(" ALL-TIME TOTALS ")),
            right[3],
        );
        let footer = if let Some(input) = &self.input {
            format!(
                "New project: {input}▏\nEnter: create · Esc: cancel\n{}",
                self.message
            )
        } else {
            format!(
                "N new · ↑↓/JK open · Space start/pause · S stop\nP Pomodoro · B include/exclude breaks · Q save & quit\n{}",
                self.message
            )
        };
        f.render_widget(
            Paragraph::new(footer).style(Style::default().fg(CYAN)),
            rows[2],
        );
    }
}
fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .title(title)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BLUE))
}
fn main() -> io::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "worktimeTUI — CyberNord project timer\nUsage: worktimeTUI [--data-dir PATH]\nN new project | arrows select | Space start/pause | S stop\nP Pomodoro | B count breaks | Q save and quit"
        );
        return Ok(());
    }
    let dir = match args.as_slice() {
        [] => std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .ok_or_else(|| io::Error::other("Set HOME or XDG_DATA_HOME, or use --data-dir"))?
            .join("worktimeTUI"),
        [flag, path] if flag == "--data-dir" => PathBuf::from(path),
        _ => return Err(io::Error::other("Usage: worktimeTUI [--data-dir PATH]")),
    };
    let (store, data) = Store::open(&dir)?;
    let mut app = App {
        data,
        message: "Ready. Timers reopen paused; downtime is never added.".into(),
        ..Default::default()
    };
    ratatui::run(|terminal| {
        execute!(io::stdout(), EnableMouseCapture)?;
        let result = run(terminal, &mut app, &store);
        let cleanup = execute!(io::stdout(), DisableMouseCapture);
        result.and(cleanup)
    })
}
fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App, store: &Store) -> io::Result<()> {
    let mut last = Instant::now();
    let mut saved = last;
    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(last).as_millis() as u64;
        last += Duration::from_millis(elapsed);
        if let Some(p) = app.data.projects.get_mut(app.selected) {
            app.timer.advance(elapsed, p, app.data.pomodoro);
        }
        terminal.draw(|f| app.draw(f))?;
        let mut changed = false;
        let mut quit = false;
        if event::poll(Duration::from_millis(100))? {
            // Account for the wait before an input can switch projects or pause.
            let now = Instant::now();
            let elapsed = now.duration_since(last).as_millis() as u64;
            last += Duration::from_millis(elapsed);
            if let Some(p) = app.data.projects.get_mut(app.selected) {
                app.timer.advance(elapsed, p, app.data.pomodoro);
            }
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    quit = app.key(key);
                    changed = true;
                }
                Event::Mouse(m)
                    if app.input.is_none() && m.kind == MouseEventKind::Down(MouseButton::Left) =>
                {
                    let pos = Position::new(m.column, m.row);
                    if app.button_area.contains(pos) {
                        app.toggle();
                    }
                    let inner = app.project_area.inner(Margin::new(1, 1));
                    if inner.contains(pos) {
                        app.select(app.list.offset() + (m.row - inner.y) as usize);
                    }
                    changed = true;
                }
                _ => (),
            }
        }
        if quit || changed || saved.elapsed() >= Duration::from_secs(1) {
            store.save(&app.data)?;
            saved = Instant::now();
        }
        if quit {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn render_and_switch_projects() {
        let mut app = App::default();
        app.data.projects.push(Project {
            name: "Alpha".into(),
            ..Default::default()
        });
        app.data.projects.push(Project {
            name: "Beta".into(),
            ..Default::default()
        });
        app.toggle();
        app.timer.advance(1000, &mut app.data.projects[0], false);
        app.select(1);
        assert!(!app.timer.running);
        assert_eq!(app.data.projects[0].focus_ms, 1000);
        assert_eq!(app.data.projects[1].focus_ms, 0);
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| app.draw(f)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("worktimeTUI"));
        assert!(text.contains("Beta"));
        assert!(text.contains("EXCLUDED"));
    }
}
