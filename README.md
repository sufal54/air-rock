
# 🪨 air-rock

**A Lightweight Linux Wireless Interface & Wi-Fi Auditing Toolkit Written in Rust.**

`air-rock` is a modular wireless auditing toolkit. It provides wireless interface management, network scanning, wireless testing, and offline capture-file analysis through command-line tools.

Built with Rust, `air-rock` aims to provide a lightweight, fast, and user-friendly wireless auditing experience on Linux.

---

## ✨ Features

- 🪨 **air-rock** — Wireless interface management.
- 📡 **air-scan** — Wireless network discovery and scanning.
- ⚡ **air-thorw** — Wireless testing module.
- 🔓 **air-crack** — Offline capture analysis with wordlist and key-search modes.
- 🔄 Wireless interface management.
- 📶 Wireless channel configuration.
- 🎯 Optional AP and client identification.
- 📁 PCAP file input support.
- 🦀 Built with Rust.

---

## 🧩 Modules

### 🪨 air-rock — Wireless Interface Manager

Inspired by `airmon-ng`, this module manages Linux wireless interfaces.

#### Commands

| Command | Description |
|---|---|
| `start` | Start wireless interface management |
| `stop` | Stop wireless interface management |
| `down` | Bring an interface down |
| `up` | Bring an interface up |
| `info` | Display interface information |

#### Usage

```bash
air-rock start wlan0 # Set to debug 
air-rock stop wlan0 # Set to manage
air-rock down wlan0 # Down the interface
air-rock up wlan0 # Up the interface
air-rock info wlan0 # Info about interface
```

---

### 📡 air-scan — Wireless Scanner

A wireless network discovery and monitoring module.

#### Arguments

| Argument | Description |
|---|---|
| `-i, --interface` | Wireless interface |
| `-b, --bssid` | Optional Access point BSSID |
| `-t, --target` | Optional client identifier |
| `-c, --channel` | Optional Wireless channel |
| `-o, --handshake` | Optional capture-file path |

#### Usage

```bash
# Scan across all frequencies
air-scan -i wlan0

# Listen on a specific frequency/channel
air-scan -i wlan0 -b ff:ff:ff:ff:ff:ff -c 1

# Listen on a specific channel and save captured packets
air-scan -i wlan0 -b ff:ff:ff:ff:ff:ff -c 1 -o capture.pcap

# Listen for a specific target
air-scan -i wlan0 -b ff:ff:ff:ff:ff:ff -t 00:00:00:00:00:00 -c 1

# Listen for a specific target and save captured packets
air-scan -i wlan0 -b ff:ff:ff:ff:ff:ff -t 00:00:00:00:00:00 -c 1 -o capture.pcap
```

Display help:

```bash
air-scan --help
```

---
### ⚡ air-thorw — Wireless Testing

A configurable wireless testing module with wireless discovery and access-point testing modes.

#### Arguments

| Argument | Description |
|---|---|
| `-i, --interface` | Wireless interface |
| `-f, --channel` | Wireless channel |
| `-c, --client` | Optional client MAC address |
| `-a, --ap` | Optional access point BSSID |
| `-g, --gap` | Optional interval configuration in milliseconds |
| `-r, --repeat` | Optional repetition count |
| `--ssid` | SSID required for access-point mode |

#### Subcommands

| Command | Description |
|---|---|
| `disc` | Wireless discovery mode |
| `ap` | Access-point testing mode |

#### Usage

```bash
# Start wireless discovery
air-thorw disc -i wlan0 -f 1

# Discover with a specific access point
air-thorw disc -i wlan0 -f 1 -a AA:BB:CC:DD:EE:FF

# Discover with a specific client
air-thorw disc -i wlan0 -f 1 -a AA:BB:CC:DD:EE:FF -c 11:22:33:44:55:66

# Start access-point testing with a specified SSID
air-thorw ap -i wlan0 -f 1 --ssid "TestNetwork"
```
---


# 🔐 air-crack — Wireless Password Auditing

A wireless password auditing module with wordlist and bruteforce testing modes.

#### Arguments

| Argument | Description |
|---|---|
| `-p, --pcap` | Path to the PCAP capture file |
| `-w, --wordlist-path` | Path to the wordlist file (wordlist mode) |
| `-k, --keys` | Key character set or key specification (bruteforce mode) |
| `-l, --low` | Minimum key length (bruteforce mode) |
| `-m, --max` | Maximum key length (bruteforce mode) |

#### Subcommands

| Command | Description |
|---|---|
| `wordlist` | Wordlist-based password auditing mode |
| `bruteforce` | Configurable key-space testing mode |

#### Usage

```bash
# Run wordlist-based auditing on an authorized capture
air-crack -p capture.pcap W -w wordlist.txt

# Display bruteforce mode 
air-crack -p capture.pcap B -k "abc" -l 2 -m 3

# Display general help
air-crack --help
```

---

# Disclaimer

This module is intended for authorized wireless security auditing, research, and educational purposes.

Only analyze captures and test networks for which you have explicit authorization.

## 🛠️ Requirements

- Linux operating system
- Rust toolchain
- Cargo package manager
- Compatible wireless network adapter
- Supported wireless drivers
- PCAP-compatible capture files
- Appropriate system permissions

---

## 🦀 Built With

- **Rust** — Core programming language.
- **Clap** — Command-line argument parsing.
- **Linux Wireless APIs** — Wireless interface management.
- **PCAP** — Packet capture file support.

---

## 🔐 Responsible Usage

`air-rock` is developed for:

- Authorized wireless security auditing.
- Personal network testing.
- Offline capture analysis.
- Wireless security research.
- Educational purposes.
- Isolated security laboratories.

Only test networks and capture files that you own or have explicit permission to assess.

Unauthorized access, disruption, or interference with third-party networks is prohibited.

The developers are not responsible for misuse or damage caused by this software.

---

## 📜 License

This project is licensed under the MIT License.

See [LICENSE](LICENSE) for details.

---

## ⭐ Support

If you find `air-rock` useful, consider giving the repository a ⭐ on GitHub.

Contributions, suggestions, and improvements are welcome.

---

<div align="center">

### 🪨 air-rock

**Monitor. Scan. Test. Analyze.**

*Built with ❤️ and Rust.*

</div>
