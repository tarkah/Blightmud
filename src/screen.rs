use crate::ansi::*;
use std::collections::VecDeque;
use std::io::{stdout, Stdout, Write};
use termion::{
    color,
    raw::{IntoRawMode, RawTerminal},
    screen::AlternateScreen,
};
use textwrap;

struct ScrollData(bool, usize);
const OUTPUT_START_LINE: u16 = 2;

pub struct Screen {
    screen: AlternateScreen<RawTerminal<Stdout>>,
    width: u16,
    _height: u16,
    output_line: u16,
    prompt_line: u16,
    history: VecDeque<String>,
    scroll_data: ScrollData,
}

impl Screen {
    pub fn new() -> Self {
        let screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
        let (width, height) = termion::terminal_size().unwrap();

        let output_line = height - 2;
        let prompt_line = height;

        Self {
            screen,
            width,
            _height: height,
            output_line,
            prompt_line,
            history: VecDeque::with_capacity(1024),
            scroll_data: ScrollData(false, 0),
        }
    }

    pub fn setup(&mut self) {
        self.reset();

        // Get params in case screen resized
        let (width, height) = termion::terminal_size().unwrap();
        self.width = width;
        self._height = height;
        self.output_line = height - 2;
        self.prompt_line = height;

        write!(
            self.screen,
            "{}{}",
            ScrollRegion(OUTPUT_START_LINE, self.output_line),
            DisableOriginMode
        )
        .unwrap(); // Set scroll region, non origin mode
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, 1),
            termion::clear::CurrentLine,
            color::Fg(color::Green),
        )
        .unwrap();
        write!(self.screen, "{:=<1$}", "", self.width as usize).unwrap(); // Print separator
        write!(self.screen, "{}", color::Fg(color::Reset)).unwrap();
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, self.output_line + 1),
            termion::clear::CurrentLine,
            color::Fg(color::Green),
        )
        .unwrap();
        write!(self.screen, "{:_<1$}", "", self.width as usize).unwrap(); // Print separator
        write!(self.screen, "{}", color::Fg(color::Reset)).unwrap();
        self.screen.flush().unwrap();
    }

    pub fn reset(&mut self) {
        write!(self.screen, "{}{}", termion::clear::All, ResetScrollRegion).unwrap();
    }

    pub fn print_prompt(&mut self, prompt: &str) {
        self.append_to_history(prompt);
        if !self.scroll_data.0 {
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, self.output_line),
                termion::scroll::Up(1),
                prompt.trim_end(),
                termion::cursor::Goto(1, self.prompt_line),
            )
            .unwrap();
        }
    }

    pub fn print_prompt_input(&mut self, input: &str) {
        let mut input = input;
        while input.len() >= self.width as usize {
            let (_, last) = input.split_at(self.width as usize);
            input = last;
        }
        write!(
            self.screen,
            "{}{}{}",
            termion::cursor::Goto(1, self.prompt_line),
            termion::clear::CurrentLine,
            input,
        )
        .unwrap();
    }

    pub fn print_output(&mut self, line: &str) {
        if line.trim().is_empty() {
            self.print_line(&line);
        } else {
            for line in textwrap::wrap_iter(line, self.width as usize) {
                self.print_line(&line);
            }
        }
    }

    fn print_line(&mut self, line: &str) {
        self.append_to_history(&line);
        if !self.scroll_data.0 {
            write!(
                self.screen,
                "{}{}{}{}",
                termion::cursor::Goto(1, self.output_line),
                termion::scroll::Up(1),
                &line,
                termion::cursor::Goto(1, self.prompt_line)
            )
            .unwrap();
        }
    }

    pub fn print_send(&mut self, send: &str) {
        self.print_output(&format!(
            "{}> {}{}",
            color::Fg(color::LightYellow),
            send,
            color::Fg(color::Reset)
        ));
    }

    pub fn print_info(&mut self, output: &str) {
        self.print_output(&format!("[**] {}", output));
    }

    pub fn print_error(&mut self, output: &str) {
        self.print_output(&format!(
            "{}[!!] {}{}",
            color::Fg(color::Red),
            output,
            color::Fg(color::Reset)
        ));
    }

    pub fn scroll_up(&mut self) {
        let output_range: usize = self.output_line as usize - OUTPUT_START_LINE as usize;
        if self.history.len() > output_range as usize {
            if !self.scroll_data.0 {
                self.scroll_data.0 = true;
                self.scroll_data.1 = self.history.len() - output_range;
            }
            self.scroll_data.0 = true;
            self.scroll_data.1 -= self.scroll_data.1.min(5);
            self.draw_scroll();
        }
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_data.0 {
            let output_range: i32 = self.output_line as i32 - OUTPUT_START_LINE as i32;
            let max_start_index: i32 = self.history.len() as i32 - output_range;
            let new_start_index = self.scroll_data.1 + 5;
            if new_start_index >= max_start_index as usize {
                self.reset_scroll();
            } else {
                self.scroll_data.1 = new_start_index;
                self.draw_scroll();
            }
        }
    }

    fn draw_scroll(&mut self) {
        let output_range = self.output_line - OUTPUT_START_LINE + 1;
        for i in 0..output_range {
            let index = self.scroll_data.1 + i as usize;
            let line_no = OUTPUT_START_LINE + i;
            write!(
                self.screen,
                "{}{}{}",
                termion::cursor::Goto(1, line_no),
                termion::clear::CurrentLine,
                self.history[index],
            )
            .unwrap();
        }
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_data.0 = false;
        let output_range = self.output_line - OUTPUT_START_LINE + 1;
        let output_start_index = self.history.len() as i32 - output_range as i32;
        if output_start_index >= 0 {
            let output_start_index = output_start_index as usize;
            for i in 0..output_range {
                let index = output_start_index + i as usize;
                let line_no = OUTPUT_START_LINE + i;
                write!(
                    self.screen,
                    "{}{}{}",
                    termion::cursor::Goto(1, line_no),
                    termion::clear::CurrentLine,
                    self.history[index],
                )
                .unwrap();
            }
        } else {
            for line in &self.history {
                write!(
                    self.screen,
                    "{}{}{}",
                    termion::cursor::Goto(1, self.output_line),
                    termion::scroll::Up(1),
                    line,
                )
                .unwrap();
            }
        }
    }

    pub fn flush(&mut self) {
        self.screen.flush().unwrap();
    }

    fn append_to_history(&mut self, line: &str) {
        let lines = line.split("\r\n");
        for line in lines {
            self.history.push_back(String::from(line));
        }
        while self.history.len() >= self.history.capacity() {
            self.history.pop_back();
        }
    }
}
