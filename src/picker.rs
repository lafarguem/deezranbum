use crate::storage::Album;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{collections::HashSet, io};

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn score_all(
    query: &str,
    haystacks: &[String],
    matcher: &mut Matcher,
    buf: &mut Vec<char>,
) -> Vec<usize> {
    if query.is_empty() {
        return (0..haystacks.len()).collect();
    }

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

    let mut scored: Vec<(u32, usize)> = haystacks
        .iter()
        .enumerate()
        .filter_map(|(i, hay)| {
            pattern
                .score(Utf32Str::new(hay, buf), matcher)
                .map(|s| (s, i))
        })
        .collect();

    scored.sort_by_key(|b| std::cmp::Reverse(b.0));
    scored.into_iter().map(|(_, i)| i).collect()
}

pub fn pick(albums: &[Album], initial_selected: Option<&HashSet<u64>>) -> io::Result<Vec<Album>> {
    if albums.is_empty() {
        return Ok(Vec::new());
    }

    let haystacks: Vec<String> = albums
        .iter()
        .map(|a| format!("{} {}", a.artist.name, a.title))
        .collect();

    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf: Vec<char> = Vec::new();

    let mut query = String::new();
    let mut selected: HashSet<u64> = initial_selected.cloned().unwrap_or(HashSet::new());
    let mut cursor: usize = 0;
    let mut results: Vec<usize> = (0..albums.len()).collect();
    let mut dirty = false;

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let chosen_ids: Vec<u64> = loop {
        if dirty {
            results = score_all(&query, &haystacks, &mut matcher, &mut buf);
            if results.is_empty() {
                cursor = 0;
            } else if cursor >= results.len() {
                cursor = results.len() - 1;
            }
            dirty = false;
        }

        let mut list_state = ListState::default();
        if !results.is_empty() {
            list_state.select(Some(cursor));
        }

        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ])
                .split(frame.area());

            let input = Paragraph::new(format!("> {}", query))
                .block(Block::default().borders(Borders::ALL).title("search"));
            frame.render_widget(input, chunks[0]);

            let status = Paragraph::new(format!(
                "selected: {} | matches: {}/{}   [tab] select+down  [space] select  [enter] confirm  [esc] cancel",
                selected.len(),
                results.len(),
                albums.len()
            ))
            .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(status, chunks[1]);

            let items: Vec<ListItem> = results
                .iter()
                .map(|&i| {
                    let a = &albums[i];
                    let mark = if selected.contains(&a.id) { "*" } else { " " };
                    let line = Line::from(vec![
                        Span::styled(format!("{mark} "), Style::default().fg(Color::Yellow)),
                        Span::raw(a.title.clone()),
                        Span::styled(" — ", Style::default().fg(Color::DarkGray)),
                        Span::styled(a.artist.name.clone(), Style::default().fg(Color::Cyan)),
                    ]);
                    ListItem::new(line)
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            frame.render_stateful_widget(list, chunks[2], &mut list_state);
        })?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match key.code {
            KeyCode::Esc => break Vec::new(),

            KeyCode::Enter => {
                if selected.is_empty() {
                    if let Some(&i) = results.get(cursor) {
                        break vec![albums[i].id];
                    }
                    break Vec::new();
                }
                break selected.into_iter().collect();
            }

            KeyCode::Up => {
                cursor = cursor.saturating_sub(1);
            }
            KeyCode::Down => {
                if !results.is_empty() {
                    cursor = (cursor + 1).min(results.len() - 1);
                }
            }

            KeyCode::Tab => {
                if let Some(&i) = results.get(cursor) {
                    let id = albums[i].id;
                    if !selected.insert(id) {
                        selected.remove(&id);
                    }
                    if !results.is_empty() {
                        cursor = (cursor + 1).min(results.len() - 1);
                    }
                }
            }

            KeyCode::Char(' ') => {
                if let Some(&i) = results.get(cursor) {
                    let id = albums[i].id;
                    if !selected.insert(id) {
                        selected.remove(&id);
                    }
                }
            }

            KeyCode::Backspace => {
                if query.pop().is_some() {
                    dirty = true;
                }
            }

            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if !query.is_empty() {
                    query.clear();
                    dirty = true;
                }
            }

            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                break Vec::new();
            }

            KeyCode::Char(c) => {
                query.push(c);
                dirty = true;
            }

            _ => {}
        }
    };

    if chosen_ids.is_empty() {
        return Ok(Vec::new());
    }

    let id_set: HashSet<u64> = chosen_ids.iter().copied().collect();
    Ok(albums
        .iter()
        .filter(|a| id_set.contains(&a.id))
        .cloned()
        .collect())
}
