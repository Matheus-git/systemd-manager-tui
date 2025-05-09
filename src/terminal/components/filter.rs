use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Paragraph},
    Frame,
};
use std::sync::mpsc::Sender;

use crate::terminal::app::{Actions, AppEvent};

pub struct Filter {
    pub input: String,
    character_index: usize,
    pub input_mode: InputMode,
    sender: Sender<AppEvent>,
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

impl Filter {
    pub const fn new(sender: Sender<AppEvent>) -> Self {
        Self {
            sender,
            input: String::new(),
            input_mode: InputMode::Normal,
            character_index: 0,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn submit_message(&mut self) {
        self.sender
            .send(AppEvent::Action(Actions::Filter(self.input.clone())))
            .unwrap();
        self.sender
            .send(AppEvent::Action(Actions::UpdateIgnoreListKeys(false)))
            .unwrap();
        self.input_mode = InputMode::Normal
    }

    pub fn on_key_event(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('i') => {
                    self.sender
                        .send(AppEvent::Action(Actions::UpdateIgnoreListKeys(true)))
                        .unwrap();
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Esc => {
                    self.input = String::new();
                    self.sender
                        .send(AppEvent::Action(Actions::Filter(self.input.clone())))
                        .unwrap();
                    self.sender
                        .send(AppEvent::Action(Actions::UpdateIgnoreListKeys(false)))
                        .unwrap();
                }
                _ => {}
            },
            InputMode::Editing if key.kind == KeyEventKind::Press => {
                match key.code {
                    KeyCode::Enter => self.submit_message(),
                    KeyCode::Char(to_insert) => self.enter_char(to_insert),
                    KeyCode::Backspace => self.delete_char(),
                    KeyCode::Left => self.move_cursor_left(),
                    KeyCode::Right => self.move_cursor_right(),
                    KeyCode::Esc => {
                        self.sender
                            .send(AppEvent::Action(Actions::UpdateIgnoreListKeys(false)))
                            .unwrap();
                        self.input_mode = InputMode::Normal;
                    }
                    _ => {}
                }
                self.sender
                    .send(AppEvent::Action(Actions::Filter(self.input.clone())))
                    .unwrap();
            }
            InputMode::Editing => {}
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let vertical = Layout::vertical([Constraint::Length(1), Constraint::Length(3)]);
        let [help_area, input_area] = vertical.areas(area);

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec!["Press ".into(), "i".bold(), " to start filtering.".into()],
                Style::default(),
            ),
            InputMode::Editing => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop filtering, ".into(),
                    "Enter".bold(),
                    " to submit filter".into(),
                ],
                Style::default(),
            ),
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);
        match self.input_mode {
            InputMode::Normal => {}
            #[allow(clippy::cast_possible_truncation)]
            InputMode::Editing => frame.set_cursor_position(Position::new(
                input_area.x + self.character_index as u16 + 1,
                input_area.y + 1,
            )),
        }
    }
}
