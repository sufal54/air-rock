use std::{
    collections::HashMap,
    io::{self, Stdout, Write, stdout},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use airctl::{
    af_eth::{PacketInfo, RadioInfo, Security},
    helper::freq_to_channel,
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::Print,
    terminal::{self, ClearType},
};

pub struct TerminalManager {
    pub stdout: Stdout,
}

impl TerminalManager {
    pub fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;

        execute!(
            stdout(),
            cursor::Hide,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0),
        )?;

        stdout().flush()?;

        Ok(Self {
            stdout: io::stdout(),
        })
    }

    pub fn clear(&self) -> io::Result<()> {
        execute!(stdout(), cursor::Show)?;
        stdout().flush()?;
        println!();

        Ok(())
    }

    pub fn print_str(&mut self, string: String) {
        let _ = queue!(self.stdout, Print(string));
    }

    pub fn is_exit(&self, terminal_running: &Arc<AtomicBool>) -> bool {
        if event::poll(Duration::from_millis(10)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    terminal_running.store(false, Ordering::Relaxed);
                    return true;
                }
            }
        }

        false
    }

    pub fn scaning(
        &mut self,
        terminal_running: Arc<AtomicBool>,
        terminal_aps: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>>,
        terminal_connected: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>>,
        terminal_live: Arc<RwLock<HashMap<String, (RadioInfo, PacketInfo)>>>,
    ) -> io::Result<()> {
        // Client table scroll position.
        let mut client_scroll: usize = 0;

        while terminal_running.load(Ordering::Relaxed) {
            // INPUT

            while event::poll(Duration::from_millis(0)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    match key.code {
                        // Ctrl+C
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            terminal_running.store(false, Ordering::Relaxed);
                            break;
                        }

                        // Scroll down
                        KeyCode::Down => {
                            client_scroll = client_scroll.saturating_add(1);
                        }

                        // Scroll up
                        KeyCode::Up => {
                            client_scroll = client_scroll.saturating_sub(1);
                        }

                        // Page down
                        KeyCode::PageDown => {
                            client_scroll = client_scroll.saturating_add(5);
                        }

                        // Page up
                        KeyCode::PageUp => {
                            client_scroll = client_scroll.saturating_sub(5);
                        }

                        // Home
                        KeyCode::Home => {
                            client_scroll = 0;
                        }

                        _ => {}
                    }
                }
            }

            if !terminal_running.load(Ordering::Relaxed) {
                break;
            }

            // CLEAR SCREEN

            execute!(
                self.stdout,
                cursor::MoveTo(0, 0),
                terminal::Clear(ClearType::FromCursorDown)
            )?;

            let (_, height) = terminal::size()?;

            // AP TABLE

            let mut y: u16 = 2;

            queue!(
                self.stdout,
                cursor::MoveTo(0, y),
                Print(format!(
                    " {:<18} {:>5} {:>4} {:<9} {:<32}",
                    "BSSID", "PWR", "CH", "SECURITY", "ESSID"
                )),
            )?;

            y += 1;

            queue!(
                self.stdout,
                cursor::MoveTo(0, y),
                Print(format!(" {}", "-".repeat(72))),
            )?;

            y += 1;

            // AP ROWS

            {
                let aps = terminal_aps.read().unwrap();

                for ap in aps.values() {
                    if y >= height {
                        break;
                    }

                    let channel =
                        ap.1.beacon_channel
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "-".to_string());

                    let ssid = ap.1.ssid.as_deref().unwrap_or("<hidden>");

                    // Safe UTF-8 truncation.
                    let ssid: String = ssid.chars().take(32).collect();

                    queue!(
                        self.stdout,
                        cursor::MoveTo(0, y),
                        Print(format!(
                            " {:<18} {:>5} {:>4} {:<9} {:<32}",
                            ap.1.bssid,
                            ap.0.signal_power.unwrap_or(-100),
                            channel,
                            ap.1.security,
                            ssid,
                        )),
                    )?;

                    y += 1;
                }
            }

            // CLIENT TABLE

            y += 2;

            let client_header_y = y;

            queue!(
                self.stdout,
                cursor::MoveTo(0, y),
                Print(format!(
                    " {:<18}  {:<18} {:>5} {:>4} {:<9}",
                    "CLIENT", "AP", "PWR", "CH", "SECURITY"
                )),
            )?;

            y += 1;

            queue!(
                self.stdout,
                cursor::MoveTo(0, y),
                Print(format!(" {}", "-".repeat(72))),
            )?;

            y += 1;

            // COLLECT CLIENTS

            enum ClientRow {
                Connected {
                    mac: String,
                    radio: RadioInfo,
                    packet: PacketInfo,
                },

                Live {
                    mac: String,
                    radio: RadioInfo,
                    packet: PacketInfo,
                },
            }

            let mut client_rows: Vec<ClientRow> = Vec::new();

            // Connected clients.
            {
                let clients = terminal_connected.read().unwrap();

                for (client_mac, client) in clients.iter() {
                    client_rows.push(ClientRow::Connected {
                        mac: client_mac.clone(),
                        radio: client.0.clone(),
                        packet: client.1.clone(),
                    });
                }
            }

            // Probe/live clients.
            // {
            //     let clients = terminal_live.read().unwrap();

            //     for (client_mac, client) in clients.iter() {
            //         client_rows.push(ClientRow::Live {
            //             mac: client_mac.clone(),
            //             radio: client.0.clone(),
            //             packet: client.1.clone(),
            //         });
            //     }
            // }

            // CALCULATE VISIBLE CLIENT ROWS

            //
            // Keep some space at the bottom for the status line.
            //
            let status_space: u16 = 4;

            let available_rows = height.saturating_sub(y).saturating_sub(status_space) as usize;

            let total_clients = client_rows.len();

            // Maximum valid scroll position.
            let max_scroll = total_clients.saturating_sub(available_rows);

            // Clamp scroll position.
            if client_scroll > max_scroll {
                client_scroll = max_scroll;
            }

            // DRAW CLIENTS

            let visible_clients = client_rows.iter().skip(client_scroll).take(available_rows);

            for client in visible_clients {
                if y >= height.saturating_sub(status_space) {
                    break;
                }

                match client {
                    ClientRow::Connected { mac, radio, packet } => {
                        let channel =
                            freq_to_channel(radio.frequency.unwrap_or(0).into()).unwrap_or(0);

                        let security = match terminal_aps.read().unwrap().get(&packet.transmitter) {
                            Some((_, packet_info)) => packet_info.security,
                            None => Security::Unknown,
                        };

                        queue!(
                            self.stdout,
                            cursor::MoveTo(0, y),
                            Print(format!(
                                " {:<18}  {:<18} {:>5} {:>4} {:<9}",
                                mac,
                                packet.transmitter,
                                radio.signal_power.unwrap_or(-100),
                                channel,
                                security,
                            )),
                        )?;
                    }

                    ClientRow::Live { mac, radio, .. } => {
                        queue!(
                            self.stdout,
                            cursor::MoveTo(0, y),
                            Print(format!(
                                " {:<18}  {:<18} {:>5} {:>4} {:<9}",
                                mac,
                                "Not Connected",
                                radio.signal_power.unwrap_or(-100),
                                "PROBE",
                                "LIVE",
                            )),
                        )?;
                    }
                }

                y += 1;
            }

            // SCROLL INDICATOR

            if total_clients > available_rows && available_rows > 0 {
                let scroll_text = format!(
                    " [{}/{}] ↑↓ Scroll  PgUp/PgDn  Home",
                    client_scroll + 1,
                    max_scroll + 1
                );

                let indicator_y = height.saturating_sub(3);

                queue!(
                    self.stdout,
                    cursor::MoveTo(0, indicator_y),
                    Print(scroll_text),
                )?;
            }

            // STATUS

            let aps_count = terminal_aps.read().unwrap().len();
            let connected_count = terminal_connected.read().unwrap().len();

            let status_y = height.saturating_sub(1);

            queue!(
                self.stdout,
                cursor::MoveTo(0, status_y),
                Print(format!(
                    " APs: {}    Connected: {}",
                    aps_count, connected_count,
                )),
            )?;

            // FLUSH

            self.stdout.flush()?;

            // Small refresh interval.
            thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }
}

impl Drop for TerminalManager {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();

        let _ = execute!(
            stdout(),
            cursor::Show,
            // terminal::Clear(ClearType::All),
            // cursor::MoveTo(0, 0),
        );

        let _ = stdout().flush();

        println!();
    }
}
