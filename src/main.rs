use std::{io::stdout, time::Duration};

use bluer::Address;
use device_stream::DeviceStream;
use sony_protocol::v1::{AncMode, AncPayload, PacketContent, PayloadCommand1};
use sony_rs::Device;
use tokio_stream::StreamExt;

use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{Event as CrosstermEvent, KeyCode, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerSmartWidget};

mod device_stream;

enum UiDeviceBattery {
    Single(u8),
    Dual((u8, u8)),
}

pub struct UiDevice {
    address: Address,
    device: Device,
    anc_mode: Option<AncPayload>,
    battery_device: Option<UiDeviceBattery>,
    battery_case: Option<u8>,
}

struct App {
    stream: device_stream::DeviceStream,
    quit: bool,
    show_logs: bool,
}

impl App {
    pub async fn new() -> Self {
        let sony_explorer = sony_rs::DeviceExplorer::start();

        Self {
            quit: false,
            show_logs: false,
            stream: DeviceStream::new(sony_explorer),
        }
    }
    pub async fn run(&mut self) -> anyhow::Result<()> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.clear()?;

        let mut reader = crossterm::event::EventStream::new();

        let mut ticker = tokio::time::interval(Duration::from_millis(16));

        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            let _ = tokio::select! {
                Some(event) = self.stream.next() => (),
                Some(evt) = reader.next() => {
                    self.on_crossterm_event(evt.unwrap()).await?;
                }
                _ = ticker.tick() => ()
            };
        }

        stdout().execute(LeaveAlternateScreen)?;
        disable_raw_mode()?;
        Ok(())
    }

    async fn on_crossterm_event(&mut self, event: CrosstermEvent) -> anyhow::Result<()> {
        match event {
            CrosstermEvent::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            self.quit = true;
                        }
                        KeyCode::Char('l') => self.show_logs = !self.show_logs,
                        KeyCode::Char('a') => {
                            if self.stream.len() > 0 {
                                let device = &self.stream[0];

                                let new_mode = if let Some(anc_mode) = &device.anc_mode {
                                    match anc_mode.anc_mode {
                                        AncMode::Off => AncPayload {
                                            anc_mode: AncMode::AmbiantMode,
                                            focus_on_voice: false,
                                            ambiant_level: 17,
                                        },
                                        AncMode::AmbiantMode => AncPayload {
                                            anc_mode: AncMode::On,
                                            focus_on_voice: false,
                                            ambiant_level: 0,
                                        },
                                        AncMode::On => AncPayload {
                                            anc_mode: AncMode::Wind,
                                            focus_on_voice: false,
                                            ambiant_level: 0,
                                        },
                                        AncMode::Wind => AncPayload {
                                            anc_mode: AncMode::Off,
                                            focus_on_voice: false,
                                            ambiant_level: 0,
                                        },
                                    }
                                } else {
                                    AncPayload {
                                        anc_mode: AncMode::On,
                                        focus_on_voice: false,
                                        ambiant_level: 0,
                                    }
                                };

                                device
                                    .device
                                    .as_ref()
                                    .send(PacketContent::Command1(
                                        PayloadCommand1::AmbientSoundControlSet(new_mode),
                                    ))
                                    .await?;
                            }
                        }
                        _ => (),
                    }
                }
            }
            CrosstermEvent::Resize(_x, _y) => {}
            _ => {}
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.size();
        if self.stream.len() > 0 {
            let device = &self.stream[0];

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(if self.show_logs {
                    [Constraint::Fill(1), Constraint::Fill(1)].as_slice()
                } else {
                    [Constraint::Fill(1)].as_slice()
                })
                .split(area);
            {
                let title_block = Block::default()
                    .borders(Borders::ALL)
                    .title(device.device.name())
                    .style(Style::default());

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ])
                    .split(title_block.inner(chunks[0]));
                frame.render_widget(title_block, area);

                let block = Block::new()
                    .title(vec!["a".red(), Span::raw("nc Mode")])
                    .borders(Borders::ALL)
                    .style(Style::default());

                let title = Paragraph::new(Text::raw(format!(
                    "{:?}",
                    device.anc_mode.as_ref().map(|f| f.anc_mode)
                )))
                .block(block);
                frame.render_widget(title, chunks[0]);
                {
                    let mut constraints = if let Some(d) = &device.battery_device {
                        match d {
                            UiDeviceBattery::Single(_) => 1,
                            UiDeviceBattery::Dual(_) => 2,
                        }
                    } else {
                        0
                    };
                    if device.battery_case.is_some() {
                        constraints += 1;
                    }
                    let chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints((0..constraints).map(|_| Constraint::Fill(1)))
                        .split(chunks[1]);

                    let index = if let Some(b) = &device.battery_device {
                        match b {
                            UiDeviceBattery::Single(level) => {
                                let block = Block::new()
                                    .title("Single")
                                    .borders(Borders::ALL)
                                    .style(Style::default());
                                let text =
                                    Paragraph::new(Text::raw(format!("{}", level))).block(block);
                                frame.render_widget(text, chunks[0]);
                                1
                            }
                            UiDeviceBattery::Dual((left, right)) => {
                                let block = Block::new()
                                    .title("Left")
                                    .borders(Borders::ALL)
                                    .style(Style::default());
                                let text =
                                    Paragraph::new(Text::raw(format!("{}", left))).block(block);
                                frame.render_widget(text, chunks[0]);

                                let block = Block::new()
                                    .title("Right")
                                    .borders(Borders::ALL)
                                    .style(Style::default());
                                let text =
                                    Paragraph::new(Text::raw(format!("{}", right))).block(block);
                                frame.render_widget(text, chunks[1]);
                                2
                            }
                        }
                    } else {
                        0
                    };

                    if let Some(case) = &device.battery_case {
                        let block = Block::new()
                            .title("Case")
                            .borders(Borders::ALL)
                            .style(Style::default());
                        let text = Paragraph::new(Text::raw(format!("{}", case))).block(block);
                        frame.render_widget(text, chunks[index]);
                    }
                }
            }
            if self.show_logs {
                let widget = TuiLoggerSmartWidget::default()
                    .style_error(Style::default().fg(Color::Red))
                    .style_debug(Style::default().fg(Color::Green))
                    .style_warn(Style::default().fg(Color::Yellow))
                    .style_trace(Style::default().fg(Color::Magenta))
                    .style_info(Style::default().fg(Color::Cyan))
                    .output_separator(':')
                    .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
                    .output_target(true)
                    .output_file(false)
                    .output_line(false);
                frame.render_widget(widget, chunks[1]);
            }
        } else {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(if self.show_logs {
                    [Constraint::Fill(1), Constraint::Fill(1)].as_slice()
                } else {
                    [Constraint::Fill(1)].as_slice()
                })
                .split(area);

            frame.render_widget(Text::raw("No device"), chunks[0]);

            if self.show_logs {
                let widget = TuiLoggerSmartWidget::default()
                    .style_error(Style::default().fg(Color::Red))
                    .style_debug(Style::default().fg(Color::Green))
                    .style_warn(Style::default().fg(Color::Yellow))
                    .style_trace(Style::default().fg(Color::Magenta))
                    .style_info(Style::default().fg(Color::Cyan))
                    .output_separator(':')
                    .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
                    .output_target(true)
                    .output_file(false)
                    .output_line(false);
                frame.render_widget(widget, chunks[1]);
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tui_logger::tracing_subscriber_layer())
        .init();
    tui_logger::set_default_level(log::LevelFilter::Trace);

    tracing::trace!("fo");

    let mut app = App::new().await;
    app.run().await?;
    Ok(())
}
