use std::{io::stdout, time::Duration};

use bluer::Address;
use sony_protocol::v1::{AncMode, AncPayload, Packet, PacketContent, PayloadCommand1};
use sony_rs::{Device, DeviceEvent};
use tokio_stream::{wrappers::BroadcastStream, StreamExt, StreamMap};
use tracing::debug;

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
    widgets::{Block, Borders, Paragraph, Widget},
    Frame, Terminal,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerSmartWidget};

enum UiDeviceBattery {
    Single(u8),
    Dual((u8, u8)),
}

struct UiDevice {
    address: Address,
    device: Device,
    anc_mode: Option<AncPayload>,
    battery_device: Option<UiDeviceBattery>,
    battery_case: Option<u8>,
}

struct App {
    devices: Vec<UiDevice>,
    quit: bool,
    show_logs: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            quit: false,
            devices: Vec::new(),
            show_logs: false,
        }
    }
}

impl App {
    pub async fn run(&mut self) -> anyhow::Result<()> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.clear()?;

        let mut event_streams = StreamMap::new();

        let sony_explorer = sony_rs::DeviceExplorer::start();

        let mut device_stream = sony_explorer.device_stream;

        let mut reader = crossterm::event::EventStream::new();

        let mut ticker = tokio::time::interval(Duration::from_millis(16));

        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            let _ = tokio::select! {
                Some(event) = device_stream.recv() => {
                    self.handle_explorer_event(event, &mut event_streams).await?;
                }
                Some((address, event)) = event_streams.next() => {
                    if let Ok(packet) = event {
                        self.handle_device_event(address, packet)?;
                    }
                }
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

    async fn handle_explorer_event(
        &mut self,
        event: DeviceEvent,
        event_streams: &mut StreamMap<Address, BroadcastStream<Packet>>,
    ) -> anyhow::Result<()> {
        debug!("{:?}", event);
        match event {
            sony_rs::DeviceEvent::DeviceAdded(d) => {
                let address = d.address();

                event_streams.insert(
                    address,
                    tokio_stream::wrappers::BroadcastStream::new(
                        d.as_ref().packets_receiver.resubscribe(),
                    ),
                );

                {
                    let d = d.clone();
                    self.devices.push(UiDevice {
                        address,
                        device: d,
                        anc_mode: None,
                        battery_device: None,
                        battery_case: None,
                    });
                }

                tokio::spawn(async move {
                    d.as_ref()
                        .send(PacketContent::Command1(
                            PayloadCommand1::AmbientSoundControlGet,
                        ))
                        .await
                        .unwrap();

                    d.as_ref()
                        .send(PacketContent::Command1(
                            PayloadCommand1::BatteryLevelRequest(
                                sony_protocol::v1::BatteryType::Single,
                            ),
                        ))
                        .await
                        .unwrap();

                    d.as_ref()
                        .send(PacketContent::Command1(
                            PayloadCommand1::BatteryLevelRequest(
                                sony_protocol::v1::BatteryType::Dual,
                            ),
                        ))
                        .await
                        .unwrap();

                    d.as_ref()
                        .send(PacketContent::Command1(
                            PayloadCommand1::BatteryLevelRequest(
                                sony_protocol::v1::BatteryType::Case,
                            ),
                        ))
                        .await
                        .unwrap();
                });
            }
            sony_rs::DeviceEvent::DeviceRemoved(_) => todo!(),
        }
        Ok(())
    }

    fn handle_device_event(&mut self, address: Address, event: Packet) -> anyhow::Result<()> {
        match event.content {
            PacketContent::Command1(c) => match c {
                PayloadCommand1::AmbientSoundControlRet(n)
                | PayloadCommand1::AmbientSoundControlNotify(n) => {
                    self.devices
                        .iter_mut()
                        .find(|c| c.address == address)
                        .unwrap()
                        .anc_mode = Some(n);
                }
                PayloadCommand1::BatteryLevelReply(b) | PayloadCommand1::BatteryLevelNotify(b) => {
                    let device = self
                        .devices
                        .iter_mut()
                        .find(|c| c.address == address)
                        .unwrap();
                    match b {
                        sony_protocol::v1::BatteryState::Single {
                            level,
                            is_charging: _,
                        } => device.battery_device = Some(UiDeviceBattery::Single(level)),
                        sony_protocol::v1::BatteryState::Case {
                            level,
                            is_charging: _,
                        } => device.battery_case = Some(level),
                        sony_protocol::v1::BatteryState::Dual {
                            level_left,
                            is_left_charging: _,
                            level_right,
                            is_right_charging: _,
                        } => {
                            device.battery_device =
                                Some(UiDeviceBattery::Dual((level_left, level_right)))
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }

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
                            if self.devices.len() > 0 {
                                let device = &self.devices[0];

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
        if self.devices.len() > 0 {
            let device = &self.devices[0];

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

    let mut app = App::default();
    app.run().await?;
    Ok(())
}
