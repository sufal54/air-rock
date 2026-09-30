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
        execute!(stdout(), cursor::Show,)?;
        stdout().flush()?;
        println!();

        Ok(())
    }

    pub fn print_str(&mut self, str: String) {
        let _ = queue!(self.stdout, Print(format!("{}", str)));
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
    ) -> std::io::Result<()> {
        while terminal_running.load(Ordering::Relaxed) {
            if self.is_exit(&terminal_running) {
                break;
            }
            // DRAW UI

            execute!(
                self.stdout,
                cursor::MoveTo(0, 0),
                terminal::Clear(ClearType::FromCursorDown)
            )?;

            // AP TABLE

            let mut y: u16 = 0;

            y += 2;

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
                    // Don't draw outside terminal
                    let (_, height) = terminal::size()?;

                    if y >= height {
                        break;
                    }

                    let channel =
                        ap.1.beacon_channel
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "-".to_string());

                    let ssid = ap.1.ssid.as_deref().unwrap_or("<hidden>");

                    // Safe UTF-8 truncation
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

            queue!(
                self.stdout,
                cursor::MoveTo(0, y),
                Print(format!(
                    " {:<18}  {:<18} {:>5} {:>4} {:<9} ",
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

            // CONNECTED CLIENTS

            {
                let clients = terminal_connected.read().unwrap();

                for (client_mac, client) in clients.iter() {
                    let (_, height) = terminal::size()?;

                    if y >= height {
                        break;
                    }

                    let channel =
                        freq_to_channel(client.0.frequency.unwrap_or(0).into()).unwrap_or(0);

                    let security = match terminal_aps.read().unwrap().get(&client.1.transmitter) {
                        Some((_, packet_info)) => packet_info.security,
                        None => Security::Unknown,
                    };

                    queue!(
                        self.stdout,
                        cursor::MoveTo(0, y),
                        Print(format!(
                            " {:<18}  {:<18} {:>5} {:>4} {:<9} ",
                            client_mac,
                            client.1.transmitter,
                            client.0.signal_power.unwrap_or(-100),
                            channel,
                            security,
                        )),
                    )?;

                    y += 1;
                }
            }

            // LIVE CLIENTS

            {
                let clients = terminal_live.read().unwrap();

                for (client_mac, client) in clients.iter() {
                    let (_, height) = terminal::size()?;

                    if y >= height {
                        break;
                    }

                    queue!(
                        self.stdout,
                        cursor::MoveTo(0, y),
                        Print(format!(
                            " {:<18} {:<18} {:<12} {:<10}",
                            client_mac, client.1.bssid, "PROBE", "LIVE",
                        )),
                    )?;

                    y += 1;
                }
            }

            // STATUS

            let (_, height) = terminal::size()?;

            let aps_count = terminal_aps.read().unwrap().len();
            let connected_count = terminal_connected.read().unwrap().len();
            let live_count = terminal_live.read().unwrap().len();

            queue!(
                self.stdout,
                cursor::MoveTo(0, height.saturating_sub(1)),
                Print(format!(
                    " APs: {}    Connected: {}    Live: {}",
                    aps_count, connected_count, live_count,
                )),
            )?;

            // Flush everything at once
            self.stdout.flush()?;

            thread::sleep(Duration::from_millis(500));
        }
        Ok(())
    }
}
impl Drop for TerminalManager {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(stdout(), cursor::Show,);
        let _ = stdout().flush();
        println!();
    }
}
